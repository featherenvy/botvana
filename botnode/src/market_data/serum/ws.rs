use std::borrow::Cow;

use serde::Deserialize;

/// Serum Websocket message
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum WsMsg<'a> {
    #[serde(rename_all = "camelCase")]
    Subscribed {
        channel: &'a str,
        markets: Vec<&'a str>,
    },
    #[serde(rename_all = "camelCase")]
    Error {
        timestamp: &'a str,
        message: &'a str,
    },
    #[serde(rename_all = "camelCase")]
    RecentTrades {
        market: &'a str,
        timestamp: &'a str,
        trades: Vec<Trade<'a>>,
    },
    #[serde(rename_all = "camelCase")]
    Trade {
        id: &'a str,
        market: &'a str,
        timestamp: &'a str,
        slot: u64,
        version: u64,
        side: &'a str,
        price: &'a str,
        size: &'a str,
        taker_account: &'a str,
        maker_account: &'a str,
    },
    #[serde(rename_all = "camelCase")]
    Quote {
        market: &'a str,
        timestamp: &'a str,
        slot: u64,
        version: u64,
        best_ask: (&'a str, &'a str),
        best_bid: (&'a str, &'a str),
    },
    #[serde(rename_all = "camelCase")]
    L2snapshot {
        market: &'a str,
        timestamp: &'a str,
        slot: u64,
        version: u64,
        asks: Box<[(&'a str, &'a str)]>,
        bids: Box<[(&'a str, &'a str)]>,
    },
    #[serde(rename_all = "camelCase")]
    L2update {
        market: &'a str,
        timestamp: &'a str,
        slot: u64,
        version: u64,
        asks: Box<[(&'a str, &'a str)]>,
        bids: Box<[(&'a str, &'a str)]>,
    },
    #[serde(rename_all = "camelCase")]
    L3snapshot {
        market: &'a str,
        timestamp: &'a str,
        slot: u64,
        version: u64,
        asks: Box<[L3order<'a>]>,
        bids: Box<[L3order<'a>]>,
    },
    #[serde(rename_all = "camelCase")]
    Open {
        market: &'a str,
        timestamp: &'a str,
        slot: u64,
        version: u64,
        order_id: &'a str,
        client_id: &'a str,
        side: &'a str,
        price: &'a str,
        size: &'a str,
        account: &'a str,
        account_slot: u64,
        fee_tier: u64,
    },
    #[serde(rename_all = "camelCase")]
    Change {
        market: &'a str,
        timestamp: &'a str,
        slot: u64,
        version: u64,
        order_id: &'a str,
        client_id: &'a str,
        side: &'a str,
        price: &'a str,
        size: &'a str,
        account: &'a str,
        account_slot: u64,
        fee_tier: u64,
    },
    #[serde(rename_all = "camelCase")]
    Fill {
        market: &'a str,
        timestamp: &'a str,
        slot: u64,
        version: u64,
        order_id: &'a str,
        client_id: &'a str,
        side: &'a str,
        price: &'a str,
        size: &'a str,
        fee_cost: f64,
        account: &'a str,
        account_slot: u64,
        fee_tier: u64,
    },
    #[serde(rename_all = "camelCase")]
    Done {
        market: &'a str,
        timestamp: &'a str,
        slot: u64,
        version: u64,
        order_id: &'a str,
        client_id: &'a str,
        side: &'a str,
        reason: &'a str,
        size_remaining: &'a str,
        account: &'a str,
        account_slot: u64,
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct L3order<'a> {
    price: &'a str,
    size: &'a str,
    side: &'a str,
    order_id: &'a str,
    client_id: &'a str,
    account_slot: u64,
    fee_tier: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Trade<'a> {
    id: &'a str,
    market: &'a str,
    timestamp: &'a str,
    slot: u64,
    version: u64,
    side: &'a str,
    price: &'a str,
    size: &'a str,
    taker_account: &'a str,
    maker_account: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_subscribed() {
        let sample = r#"{
  "type": "subscribed",
  "channel": "level2",
  "markets": ["BTC/USDC"],
  "timestamp": "2021-03-23T17:06:30.010Z"
}"#;

        serde_json::from_slice::<WsMsg>(sample.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_error() {
        let sample = r#"{
  "type": "error",
  "message": "Invalid channel provided: 'levels1'.",
  "timestamp": "2021-03-23T17:13:31.010Z"
}"#;

        serde_json::from_slice::<WsMsg>(sample.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_recent_trades() {
        let sample = r#"{
  "type": "recent_trades",
  "market": "BTC/USDC",
  "timestamp": "2021-03-24T07:05:27.377Z",
  "trades": [
    {
      "type": "trade",
      "market": "SOL/USDC",
      "timestamp": "2021-12-23T14:31:16.733Z",
      "slot": 112915164,
      "version": 3,
      "id": "3313016788894161792503559|3313035235638235438412464",
      "side": "sell",
      "price": "179.599",
      "size": "125.4",
      "takerAccount": "AAddgLu9reZCUWW1bNQFaXrCMAtwQpMRvmeusgk4pCM6",
      "makerAccount": "EpAdzaqV13Es3x4dukfjFoCrKVXnZ7y9Y76whgMHo5qx"
    }
  ]
}"#;

        serde_json::from_slice::<WsMsg>(sample.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_trade() {
        let sample = r#"{
  "type": "trade",
  "market": "SOL/USDC",
  "timestamp": "2021-12-23T14:31:16.733Z",
  "slot": 112915164,
  "version": 3,
  "id": "3313016788894161792503559|3313035235638235438412464",
  "side": "sell",
  "price": "179.599",
  "size": "125.4",
  "takerAccount": "AAddgLu9reZCUWW1bNQFaXrCMAtwQpMRvmeusgk4pCM6",
  "makerAccount": "EpAdzaqV13Es3x4dukfjFoCrKVXnZ7y9Y76whgMHo5qx"
}"#;

        serde_json::from_slice::<WsMsg>(sample.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_quote() {
        let sample = r#"{
  "type": "quote",
  "market": "BTC/USDC",
  "timestamp": "2021-03-24T07:11:57.186Z",
  "slot": 70544253,
  "version": 3,
  "bestAsk": ["55336.1", "5.0960"],
  "bestBid": ["55285.6", "7.5000"]
}"#;

        serde_json::from_slice::<WsMsg>(sample.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_l2snapshot() {
        let sample = r#"{
  "type": "l2snapshot",
  "market": "BTC/USDC",
  "timestamp": "2021-03-24T09:00:53.087Z",
  "slot": 70555623,
  "version": 3,
  "asks": [
    ["56463.3", "8.6208"],
    ["56474.3", "5.8632"],
    ["56496.4", "3.7627"]
  ],
  "bids": [
    ["56386.0", "4.8541"],
    ["56370.1", "6.8054"],
    ["56286.3", "8.6631"]
  ]
}"#;

        serde_json::from_slice::<WsMsg>(sample.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_l2update() {
        let sample = r#"{
  "type": "l2update",
  "market": "BTC/USDC",
  "timestamp": "2021-03-24T09:00:55.586Z",
  "slot": 70555627,
  "version": 3,
  "asks": [["56511.5", "7.5000"]],
  "bids": [
    ["56421.6", "0.0000"],
    ["56433.6", "5.9475"]
  ]
}"#;

        serde_json::from_slice::<WsMsg>(sample.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_l3snapshot() {
        let sample = r#"{
  "type": "l3snapshot",
  "market": "BTC/USDC",
  "timestamp": "2021-03-24T09:49:51.070Z",
  "slot": 70560748,
  "version": 3,
  "asks": [
    {
      "orderId": "10430028906948338708824594",
      "clientId": "13065347387987527730",
      "side": "sell",
      "price": "56541.3",
      "size": "4.9049",
      "account": "EXkXcPkqFwqJPXpJdTHMdvmLE282PRShqwMTteWcfz85",
      "accountSlot": 8,
      "feeTier": 3
    }
  ],
  "bids": [
    {
      "orderId": "10414533641926422683532775",
      "clientId": "1616579378239885365",
      "side": "buy",
      "price": "56457.2",
      "size": "7.5000",
      "account": "6Yqus2UYf1wSaKBE4GSLeE2Ge225THeyPcgWBaoGzx3e",
      "accountSlot": 10,
      "feeTier": 6
    }
  ]
}"#;

        serde_json::from_slice::<WsMsg>(sample.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_open() {
        let sample = r#"{
  "type": "open",
  "market": "BTC/USDC",
  "timestamp": "2021-03-24T10:14:33.967Z",
  "slot": 70563387,
  "version": 3,
  "orderId": "10395754856459386361922812",
  "clientId": "1616580865182472471",
  "side": "sell",
  "price": "56355.5",
  "size": "7.5000",
  "account": "6Yqus2UYf1wSaKBE4GSLeE2Ge225THeyPcgWBaoGzx3e",
  "accountSlot": 6,
  "feeTier": 6
}"#;

        serde_json::from_slice::<WsMsg>(sample.as_bytes()).unwrap();
    }


    #[test]
    fn test_parse_change() {
        let sample = r#"{
  "type": "change",
  "market": "BTC/USDC",
  "timestamp": "2021-03-24T10:25:21.739Z",
  "slot": 70564525,
  "version": 3,
  "orderId": "10352165200213210691454558",
  "clientId": "15125925100673159264",
  "side": "sell",
  "price": "56119.2",
  "size": "8.4494",
  "account": "EXkXcPkqFwqJPXpJdTHMdvmLE282PRShqwMTteWcfz85",
  "accountSlot": 6,
  "feeTier": 3
}"#;

        serde_json::from_slice::<WsMsg>(sample.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_fill() {
        let sample = r#"{
  "type": "fill",
  "market": "BTC/USDC",
  "timestamp": "2021-03-24T11:27:21.739Z",
  "slot": 70564527,
  "version": 3,
  "orderId": "1035216520046710691454558",
  "clientId": "151259251006473159264",
  "side": "sell",
  "price": "56119.2",
  "size": "8.4494",
  "maker": false,
  "feeCost": 15.6,
  "account": "EXkXcPkqFwqJPXpJdTHMdvmLE282PRShqwMTteWcfz85",
  "accountSlot": 6,
  "feeTier": 3
}"#;

        serde_json::from_slice::<WsMsg>(sample.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_done() {
        let sample = r#"{
  "type": "done",
  "market": "SRM/USDC",
  "timestamp": "2021-11-16T12:29:12.180Z",
  "slot": 107165458,
  "version": 3,
  "orderId": "117413526029161279193704",
  "clientId": "4796015225289787768",
  "side": "buy",
  "reason": "canceled",
  "account": "AqeHe31ZUDgEUSidkh3gEhkf7iPn8bSTJ6c8L9ymp8Vj",
  "accountSlot": 0,
  "sizeRemaining": "508.5"
}"#;

        serde_json::from_slice::<WsMsg>(sample.as_bytes()).unwrap();
    }
}
