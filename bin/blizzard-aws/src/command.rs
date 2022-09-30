use std::{
    fs,
    io::{self, Error, ErrorKind},
    path::Path,
    sync::Arc,
};

use crate::{cloudwatch as cw, flags};
use aws_manager::{self, cloudwatch, ec2, s3};

pub async fn execute(opts: flags::Options) -> io::Result<()> {
    println!("starting {} with {:?}", crate::APP_NAME, opts);

    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, opts.log_level),
    );

    let meta = fetch_metadata().await?;

    let aws_creds = load_aws_credential(&meta.region).await?;
    let ec2_manager_arc = Arc::new(aws_creds.ec2_manager.clone());
    let s3_manager_arc = Arc::new(aws_creds.s3_manager.clone());
    let cw_manager_arc = Arc::new(aws_creds.cw_manager.clone());

    let tags = fetch_tags(
        Arc::clone(&ec2_manager_arc),
        Arc::new(meta.ec2_instance_id.clone()),
    )
    .await?;

    let spec = download_spec(
        Arc::clone(&s3_manager_arc),
        &tags.s3_bucket,
        &tags.id,
        &tags.blizzardup_spec_path,
    )
    .await?;

    if !Path::new(&tags.cloudwatch_config_file_path).exists() {
        create_cloudwatch_config(&tags.id, true, &tags.cloudwatch_config_file_path)?;
    } else {
        log::warn!("skipping writing cloudwatch config (already exists)")
    }

    let handles = vec![
        // send loads asynchronously
        tokio::spawn(send_loads(
            spec.blizzard_spec.clone(),
            Arc::clone(&cw_manager_arc),
        )),
    ];

    log::info!("STEP: blocking on handles via JoinHandle");
    for handle in handles {
        handle.await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed await on JoinHandle {}", e),
            )
        })?;
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct Metadata {
    region: String,
    ec2_instance_id: String,
}

async fn fetch_metadata() -> io::Result<Metadata> {
    log::info!("STEP: fetching EC2 instance metadata...");

    let az = ec2::metadata::fetch_availability_zone()
        .await
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed fetch_availability_zone {}", e),
            )
        })?;
    log::info!("fetched availability zone {}", az);

    let reg = ec2::metadata::fetch_region()
        .await
        .map_err(|e| Error::new(ErrorKind::Other, format!("failed fetch_region {}", e)))?;
    log::info!("fetched region {}", reg);

    let ec2_instance_id = ec2::metadata::fetch_instance_id()
        .await
        .map_err(|e| Error::new(ErrorKind::Other, format!("failed fetch_instance_id {}", e)))?;
    log::info!("fetched EC2 instance Id {}", ec2_instance_id);

    let public_ipv4 = ec2::metadata::fetch_public_ipv4()
        .await
        .map_err(|e| Error::new(ErrorKind::Other, format!("failed fetch_public_ipv4 {}", e)))?;
    log::info!("fetched public ipv4 {}", public_ipv4);

    Ok(Metadata {
        region: reg,
        ec2_instance_id,
    })
}

#[derive(Debug, Clone)]
struct AwsCreds {
    ec2_manager: ec2::Manager,
    s3_manager: s3::Manager,
    cw_manager: cloudwatch::Manager,
}

async fn load_aws_credential(reg: &str) -> io::Result<AwsCreds> {
    log::info!("STEP: loading up AWS credential for region '{}'...", reg);

    let shared_config = aws_manager::load_config(Some(reg.to_string())).await?;

    let ec2_manager = ec2::Manager::new(&shared_config);
    let s3_manager = s3::Manager::new(&shared_config);
    let cw_manager = cloudwatch::Manager::new(&shared_config);

    Ok(AwsCreds {
        ec2_manager,
        s3_manager,
        cw_manager,
    })
}

#[derive(Debug, Clone)]
struct Tags {
    id: String,
    arch_type: String,
    os_type: String,
    asg_spot_instance: bool,
    node_kind: String,
    s3_bucket: String,
    cloudwatch_config_file_path: String,
    blizzardup_spec_path: String,
}

async fn fetch_tags(
    ec2_manager: Arc<ec2::Manager>,
    ec2_instance_id: Arc<String>,
) -> io::Result<Tags> {
    log::info!("STEP: fetching tags...");

    let tags = ec2_manager
        .fetch_tags(ec2_instance_id)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, format!("failed fetch_tags {}", e)))?;

    let mut fetched_tags = Tags {
        id: String::new(),
        arch_type: String::new(),
        os_type: String::new(),
        asg_spot_instance: false,
        node_kind: String::new(),
        s3_bucket: String::new(),
        cloudwatch_config_file_path: String::new(),
        blizzardup_spec_path: String::new(),
    };
    for c in tags {
        let k = c.key().unwrap();
        let v = c.value().unwrap();

        log::info!("EC2 tag key='{}', value='{}'", k, v);
        match k {
            "ID" => {
                fetched_tags.id = v.to_string();
            }
            "ARCH_TYPE" => {
                fetched_tags.arch_type = v.to_string();
            }
            "OS_TYPE" => {
                fetched_tags.os_type = v.to_string();
            }
            "ASG_SPOT_INSTANCE" => {
                fetched_tags.asg_spot_instance = v.to_string().to_lowercase().eq("true");
            }
            "NODE_KIND" => {
                fetched_tags.node_kind = v.to_string();
            }
            "S3_BUCKET_NAME" => {
                fetched_tags.s3_bucket = v.to_string();
            }
            "CLOUDWATCH_CONFIG_FILE_PATH" => {
                fetched_tags.cloudwatch_config_file_path = v.to_string();
            }
            "BLIZZARDUP_SPEC_PATH" => {
                fetched_tags.blizzardup_spec_path = v.to_string();
            }
            _ => {}
        }
    }

    assert!(!fetched_tags.id.is_empty());
    assert!(fetched_tags.node_kind.eq("worker"));
    assert!(!fetched_tags.arch_type.is_empty());
    assert!(!fetched_tags.os_type.is_empty());
    assert!(!fetched_tags.s3_bucket.is_empty());
    assert!(!fetched_tags.cloudwatch_config_file_path.is_empty());
    assert!(!fetched_tags.blizzardup_spec_path.is_empty());

    Ok(fetched_tags)
}

async fn download_spec(
    s3_manager: Arc<s3::Manager>,
    s3_bucket: &str,
    id: &str,
    blizzardup_spec_path: &str,
) -> io::Result<blizzardup_aws::Spec> {
    log::info!("STEP: downloading blizzardup spec file from S3...");

    let tmp_spec_file_path = random_manager::tmp_path(15, Some(".yaml"))?;

    let s3_manager: &s3::Manager = s3_manager.as_ref();
    s3::spawn_get_object(
        s3_manager.to_owned(),
        s3_bucket,
        &blizzardup_aws::StorageNamespace::ConfigFile(id.to_string()).encode(),
        &tmp_spec_file_path,
    )
    .await
    .map_err(|e| Error::new(ErrorKind::Other, format!("failed spawn_get_object {}", e)))?;

    let spec = blizzardup_aws::Spec::load(&tmp_spec_file_path)?;

    fs::copy(&tmp_spec_file_path, &blizzardup_spec_path)?;
    fs::remove_file(&tmp_spec_file_path)?; // "blizzard" never updates "spec" file, runs in read-only mode

    Ok(spec)
}

fn create_cloudwatch_config(
    id: &str,
    log_auto_removal: bool,
    cloudwatch_config_file_path: &str,
) -> io::Result<()> {
    log::info!("STEP: creating CloudWatch JSON config file...");

    let cw_config_manager = cw::ConfigManager {
        id: id.to_string(),
        node_kind: String::from("worker"),
        instance_system_logs: true,
        config_file_path: cloudwatch_config_file_path.to_string(),
    };
    cw_config_manager.sync(
        log_auto_removal,
        Some(vec![
            String::from("/var/log/cloud-init-output.log"),
            String::from("/var/log/blizzard.log"),
        ]),
    )
}

async fn send_loads(
    blizzard_spec: blizzardup_aws::blizzard::Spec,
    cw_manager: Arc<cloudwatch::Manager>,
) {
    log::info!(
        "start sending loads to {} endpoints",
        blizzard_spec.rpc_endpoints.len()
    );

    let _cw_manager: &cloudwatch::Manager = cw_manager.as_ref();
    // TODO: update load testing status in CloudWatch

    loop {
        log::info!("sending loads");

        break;
    }
}