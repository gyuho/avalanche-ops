use std::{
    collections::HashMap,
    io::{self, Error, ErrorKind},
    process::Command,
    string::String,
    time::Duration,
};

use log::info;
use serde::{Deserialize, Serialize};

use crate::{
    avalanche::avalanchego::api::{avax, jsonrpc},
    utils::http,
};

/// ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetbalance
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct GetBalanceResponse {
    pub jsonrpc: String,
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<GetBalanceResult>,
}

/// ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetbalance
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct GetBalanceResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unlocked: Option<u64>,
    #[serde(rename = "lockedStakeable", skip_serializing_if = "Option::is_none")]
    pub locked_stakeable: Option<u64>,
    #[serde(rename = "lockedNotStakeable", skip_serializing_if = "Option::is_none")]
    pub locked_not_stakeable: Option<u64>,
    #[serde(rename = "utxoIDs", skip_serializing_if = "Option::is_none")]
    pub utxo_ids: Option<Vec<avax::UtxoId>>,
}

impl Default for GetBalanceResult {
    fn default() -> Self {
        Self::default()
    }
}

impl GetBalanceResult {
    pub fn default() -> Self {
        Self {
            balance: None,
            unlocked: None,
            locked_stakeable: None,
            locked_not_stakeable: None,
            utxo_ids: None,
        }
    }
}

/// e.g., "platform.getBalance" on "http://[ADDR]:9650" and "/ext/bc/P" path.
/// ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetbalance
pub async fn get_balance(url: &str, path: &str, paddr: &str) -> io::Result<GetBalanceResponse> {
    let joined = http::join_uri(url, path)?;
    info!("getting balances for {} via {:?}", paddr, joined);

    let mut data = jsonrpc::Data::default();
    data.method = String::from("platform.getBalance");

    let mut params = HashMap::new();
    params.insert(String::from("address"), paddr.to_string());
    data.params = Some(params);

    let d = data.encode_json()?;

    let resp: _GetBalanceResponse = {
        if url.starts_with("https") {
            // TODO: implement this with native Rust
            info!("sending via curl --insecure");
            let mut cmd = Command::new("curl");
            cmd.arg("--insecure");
            cmd.arg("-X POST");
            cmd.arg("--header 'content-type:application/json;'");
            cmd.arg(format!("--data '{}'", d));
            cmd.arg(joined.as_str());

            let output = cmd.output()?;
            match serde_json::from_slice(&output.stdout) {
                Ok(p) => p,
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("failed to decode {}", e),
                    ));
                }
            }
        } else {
            let req = http::create_json_post(url, path, &d)?;
            let buf = match http::read_bytes(
                req,
                Duration::from_secs(5),
                url.starts_with("https"),
                false,
            )
            .await
            {
                Ok(u) => u,
                Err(e) => return Err(e),
            };
            match serde_json::from_slice(&buf) {
                Ok(p) => p,
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("failed to decode {}", e),
                    ));
                }
            }
        }
    };

    let parsed = resp.convert()?;
    Ok(parsed)
}

/// ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetbalance
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
struct _GetBalanceResponse {
    jsonrpc: String,
    id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<_GetBalanceResult>,
}

/// ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetbalance
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
struct _GetBalanceResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    balance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unlocked: Option<String>,
    #[serde(rename = "lockedStakeable", skip_serializing_if = "Option::is_none")]
    pub locked_stakeable: Option<String>,
    #[serde(rename = "lockedNotStakeable", skip_serializing_if = "Option::is_none")]
    pub locked_not_stakeable: Option<String>,
    #[serde(rename = "utxoIDs", skip_serializing_if = "Option::is_none")]
    utxo_ids: Option<Vec<avax::UtxoId>>,
}

impl _GetBalanceResponse {
    fn convert(&self) -> io::Result<GetBalanceResponse> {
        let mut result = GetBalanceResult::default();
        if self.result.is_some()
            && self
                .result
                .clone()
                .expect("unexpected None result")
                .balance
                .is_some()
        {
            let balance = self
                .result
                .clone()
                .expect("unexpected None result")
                .balance
                .expect("unexpected None balance");
            let balance = balance.parse::<u64>().unwrap();
            result.balance = Some(balance);
        }

        if self.result.is_some()
            && self
                .result
                .clone()
                .expect("unexpected None result")
                .unlocked
                .is_some()
        {
            let unlocked = self
                .result
                .clone()
                .expect("unexpected None result")
                .unlocked
                .expect("unexpected None unlocked");
            let unlocked = unlocked.parse::<u64>().unwrap();
            result.unlocked = Some(unlocked);
        }

        if self.result.is_some()
            && self
                .result
                .clone()
                .expect("unexpected None result")
                .locked_stakeable
                .is_some()
        {
            let locked_stakeable = self
                .result
                .clone()
                .expect("unexpected None result")
                .locked_stakeable
                .expect("unexpected None locked_stakeable");
            let locked_stakeable = locked_stakeable.parse::<u64>().unwrap();
            result.locked_stakeable = Some(locked_stakeable);
        }

        if self.result.is_some()
            && self
                .result
                .clone()
                .expect("unexpected None result")
                .locked_not_stakeable
                .is_some()
        {
            let locked_not_stakeable = self
                .result
                .clone()
                .expect("unexpected None result")
                .locked_not_stakeable
                .expect("unexpected None locked_not_stakeable");
            let locked_not_stakeable = locked_not_stakeable.parse::<u64>().unwrap();
            result.locked_not_stakeable = Some(locked_not_stakeable);
        }

        if self.result.is_some()
            && self
                .result
                .clone()
                .expect("unexpected None result")
                .utxo_ids
                .is_some()
        {
            let utxo_ids = self
                .result
                .clone()
                .expect("unexpected None result")
                .utxo_ids
                .expect("unexpected None utxo_ids");
            result.utxo_ids = Some(utxo_ids);
        }

        Ok(GetBalanceResponse {
            jsonrpc: self.jsonrpc.clone(),
            id: self.id,
            result: Some(result),
        })
    }
}

#[test]
fn test_convert_get_balance() {
    // ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetbalance
    let resp: _GetBalanceResponse = serde_json::from_str(
        "

{
    \"jsonrpc\": \"2.0\",
    \"result\": {
        \"balance\": \"20000000000000000\",
        \"unlocked\": \"10000000000000000\",
        \"lockedStakeable\": \"10000000000000000\",
        \"lockedNotStakeable\": \"0\",
        \"utxoIDs\": [
            {
                \"txID\": \"11111111111111111111111111111111LpoYY\",
                \"outputIndex\": 1
            },
            {
                \"txID\": \"11111111111111111111111111111111LpoYY\",
                \"outputIndex\": 0
            }
        ]
    },
    \"id\": 1
}

",
    )
    .unwrap();
    let parsed = resp.convert().unwrap();
    let expected = GetBalanceResponse {
        jsonrpc: "2.0".to_string(),
        id: 1,
        result: Some(GetBalanceResult {
            balance: Some(20000000000000000),
            unlocked: Some(10000000000000000),
            locked_stakeable: Some(10000000000000000),
            locked_not_stakeable: Some(0),
            utxo_ids: Some(vec![
                avax::UtxoId {
                    tx_id: Some(String::from("11111111111111111111111111111111LpoYY")),
                    output_index: Some(1),
                },
                avax::UtxoId {
                    tx_id: Some(String::from("11111111111111111111111111111111LpoYY")),
                    output_index: Some(0),
                },
            ]),
        }),
    };
    assert_eq!(parsed, expected);
}

/// ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetutxos
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub struct GetUtxosRequest {
    pub addresses: Vec<String>,
    pub limit: u32,
    pub encoding: String,
}

/// ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetutxos
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct GetUtxosResponse {
    pub jsonrpc: String,
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<GetUtxosResult>,
}

/// ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetutxos
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub struct EndIndex {
    pub address: String,
    pub utxo: String,
}

/// ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetutxos
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct GetUtxosResult {
    #[serde(rename = "numFetched", skip_serializing_if = "Option::is_none")]
    pub num_fetched: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utxos: Option<Vec<String>>,
    #[serde(rename = "endIndex", skip_serializing_if = "Option::is_none")]
    pub end_index: Option<EndIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

impl Default for GetUtxosResult {
    fn default() -> Self {
        Self::default()
    }
}

impl GetUtxosResult {
    pub fn default() -> Self {
        Self {
            num_fetched: None,
            utxos: None,
            end_index: None,
            encoding: None,
        }
    }
}

/// ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetutxos
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct _GetUtxosResponse {
    pub jsonrpc: String,
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<_GetUtxosResult>,
}

/// ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetutxos
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct _GetUtxosResult {
    #[serde(rename = "numFetched", skip_serializing_if = "Option::is_none")]
    pub num_fetched: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utxos: Option<Vec<String>>,
    #[serde(rename = "endIndex", skip_serializing_if = "Option::is_none")]
    pub end_index: Option<EndIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

/// e.g., "platform.getUTXOs" on "http://[ADDR]:9650" and "/ext/bc/P" path.
/// ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetutxos
pub async fn get_utxos(url: &str, path: &str, paddr: &str) -> io::Result<GetUtxosResponse> {
    let joined = http::join_uri(url, path)?;
    info!("getting UTXOs for {} via {:?}", paddr, joined);

    let mut data = DataForGetUtxos::default();
    data.method = String::from("platform.getUTXOs");

    let params = GetUtxosRequest {
        addresses: vec![paddr.to_string()],
        limit: 100,
        encoding: String::from("hex"), // don't use "cb58"
    };
    data.params = Some(params);

    let d = data.encode_json()?;

    let resp: _GetUtxosResponse = {
        if url.starts_with("https") {
            // TODO: implement this with native Rust
            info!("sending via curl --insecure");
            let mut cmd = Command::new("curl");
            cmd.arg("--insecure");
            cmd.arg("-X POST");
            cmd.arg("--header 'content-type:application/json;'");
            cmd.arg(format!("--data '{}'", d));
            cmd.arg(joined.as_str());

            let output = cmd.output()?;
            match serde_json::from_slice(&output.stdout) {
                Ok(p) => p,
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("failed to decode {}", e),
                    ));
                }
            }
        } else {
            let req = http::create_json_post(url, path, &d)?;
            let buf = match http::read_bytes(
                req,
                Duration::from_secs(5),
                url.starts_with("https"),
                false,
            )
            .await
            {
                Ok(u) => u,
                Err(e) => return Err(e),
            };
            match serde_json::from_slice(&buf) {
                Ok(p) => p,
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("failed to decode {}", e),
                    ));
                }
            }
        }
    };

    let parsed = resp.convert()?;
    Ok(parsed)
}

/// ref. https://docs.avax.network/build/avalanchego-apis/issuing-api-calls
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct DataForGetUtxos {
    pub jsonrpc: String,
    pub id: u32,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<GetUtxosRequest>,
}

impl Default for DataForGetUtxos {
    fn default() -> Self {
        Self::default()
    }
}

impl DataForGetUtxos {
    pub fn default() -> Self {
        Self {
            jsonrpc: String::from(jsonrpc::DEFAULT_VERSION),
            id: jsonrpc::DEFAULT_ID,
            method: String::new(),
            params: None,
        }
    }

    pub fn encode_json(&self) -> io::Result<String> {
        match serde_json::to_string(&self) {
            Ok(s) => Ok(s),
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("failed to serialize to JSON {}", e),
                ));
            }
        }
    }
}

impl _GetUtxosResponse {
    fn convert(&self) -> io::Result<GetUtxosResponse> {
        let mut result = GetUtxosResult::default();
        if self.result.is_some()
            && self
                .result
                .clone()
                .expect("unexpected None result")
                .num_fetched
                .is_some()
        {
            let num_fetched = self
                .result
                .clone()
                .expect("unexpected None result")
                .num_fetched
                .expect("unexpected None num_fetched");
            let num_fetched = num_fetched.parse::<u32>().unwrap();
            result.num_fetched = Some(num_fetched);
        }

        if self.result.is_some()
            && self
                .result
                .clone()
                .expect("unexpected None result")
                .utxos
                .is_some()
        {
            let utxos = self
                .result
                .clone()
                .expect("unexpected None result")
                .utxos
                .expect("unexpected None utxos");
            result.utxos = Some(utxos);
        }

        if self.result.is_some()
            && self
                .result
                .clone()
                .expect("unexpected None result")
                .end_index
                .is_some()
        {
            let end_index = self
                .result
                .clone()
                .expect("unexpected None result")
                .end_index
                .expect("unexpected None end_index");
            result.end_index = Some(end_index);
        }

        if self.result.is_some()
            && self
                .result
                .clone()
                .expect("unexpected None result")
                .encoding
                .is_some()
        {
            let encoding = self
                .result
                .clone()
                .expect("unexpected None result")
                .encoding
                .expect("unexpected None encoding");
            result.encoding = Some(encoding);
        }

        Ok(GetUtxosResponse {
            jsonrpc: self.jsonrpc.clone(),
            id: self.id,
            result: Some(result),
        })
    }
}

/// RUST_LOG=debug cargo test --package avalanche-ops --lib -- avalanche::avalanchego::api::platform::test_convert_get_utxos_empty --exact --show-output
#[test]
fn test_convert_get_utxos_empty() {
    // ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetbalance
    let resp: _GetUtxosResponse = serde_json::from_str(
        "

{
    \"jsonrpc\": \"2.0\",
    \"result\": {
        \"numFetched\": \"0\",
        \"utxos\": [],
        \"endIndex\": {
            \"address\": \"P-custom152qlr6zunz7nw2kc4lfej3cn3wk46u3002k4w5\",
            \"utxo\": \"11111111111111111111111111111111LpoYY\"
        },
        \"encoding\":\"hex\"
    },
    \"id\": 1
}

",
    )
    .unwrap();
    let parsed = resp.convert().unwrap();
    let expected = GetUtxosResponse {
        jsonrpc: "2.0".to_string(),
        id: 1,
        result: Some(GetUtxosResult {
            num_fetched: Some(0),
            utxos: Some(Vec::new()),
            end_index: Some(EndIndex {
                address: String::from("P-custom152qlr6zunz7nw2kc4lfej3cn3wk46u3002k4w5"),
                utxo: String::from("11111111111111111111111111111111LpoYY"),
            }),
            encoding: Some(String::from("hex")),
        }),
    };
    assert_eq!(parsed, expected);
}

/// RUST_LOG=debug cargo test --package avalanche-ops --lib -- avalanche::avalanchego::api::platform::test_convert_get_utxos_non_empty --exact --show-output
#[test]
fn test_convert_get_utxos_non_empty() {
    // ref. https://docs.avax.network/build/avalanchego-apis/p-chain/#platformgetbalance
    let resp: _GetUtxosResponse = serde_json::from_str(
        "

{
    \"jsonrpc\": \"2.0\",
    \"result\": {
        \"numFetched\": \"1\",
        \"utxos\": [
            \"0x000000000000000000000000000000000000000000000000000000000000000000000000000088eec2e099c6a528e689618e8721e04ae85ea574c7a15a7968644d14d54780140000000702c68af0bb1400000000000000000000000000010000000165844a05405f3662c1928142c6c2a783ef871de939b564db\"
        ],
        \"endIndex\": {
            \"address\": \"P-custom1vkzy5p2qtumx9svjs9pvds48s0hcw80f962vrs\",
            \"utxo\": \"LUC1cmcxnfNR9LdkACS2ccGKLEK7SYqB4gLLTycQfg1koyfSq\"
        },
        \"encoding\": \"hex\"
    },
    \"id\": 1
}

",
    )
    .unwrap();
    let parsed = resp.convert().unwrap();
    let expected = GetUtxosResponse {
        jsonrpc: "2.0".to_string(),
        id: 1,
        result: Some(GetUtxosResult {
            num_fetched: Some(1),
            utxos: Some(vec![
                String::from("0x000000000000000000000000000000000000000000000000000000000000000000000000000088eec2e099c6a528e689618e8721e04ae85ea574c7a15a7968644d14d54780140000000702c68af0bb1400000000000000000000000000010000000165844a05405f3662c1928142c6c2a783ef871de939b564db"),
            ]),
            end_index: Some(EndIndex {
                address: String::from("P-custom1vkzy5p2qtumx9svjs9pvds48s0hcw80f962vrs"),
                utxo: String::from("LUC1cmcxnfNR9LdkACS2ccGKLEK7SYqB4gLLTycQfg1koyfSq"),
            }),
            encoding: Some(String::from("hex")),
        }),
    };
    assert_eq!(parsed, expected);
}
