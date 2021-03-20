use agnostic::trade;
use agnostic::trade::TradeResult;
use agnostic::order::OrderWithId;
use agnostic::trading_pair;
use std::io::Read;
use std::io::Seek;
use std::io::Write;

pub struct Bookkeeper {
    trades: std::fs::File,
}

impl Bookkeeper {
    const DEAFULT_TRADES_PATH: &'static str = "trades";
    const DEFAULT_EXTENSION: &'static str = "agnostic";
    const SPLITTER: char = '|';

    pub fn new() -> std::io::Result<Bookkeeper> {
        let time = time::OffsetDateTime::now_utc();
        let filename = format!(
            "{}_{}-{}-{}_{}-{}-{}.{}",
            Self::DEAFULT_TRADES_PATH,
            time.year(),
            time.month(),
            time.day(),
            time.hour(),
            time.minute(),
            time.second(),
            Self::DEFAULT_EXTENSION
        );
        println!("{}", filename);
        let trades = std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(&filename)?;
        Ok(Bookkeeper { trades })
    }

    pub fn commit_trade(&mut self, trade: trade::Trade) {
        let trade: Trade = trade.into();
        let mut trade = serde_json::to_string(&trade).expect("Serialization error");
        trade.push(Self::SPLITTER);
        self.trades
            .seek(std::io::SeekFrom::End(0))
            .expect("Failed to seek to end");
        self.trades
            .write_all(trade.as_bytes())
            .expect("Failed to commit order");
    }

    pub fn get_all_trades(&mut self) -> Vec<Trade> {
        let mut result = String::with_capacity(100);
        self.trades
            .seek(std::io::SeekFrom::Start(0))
            .expect("Failed to seek to start");
        self.trades
            .read_to_string(&mut result)
            .expect("Failed to read trades.");
        result
            .split(Self::SPLITTER)
            .filter_map(
                |trade_json| match serde_json::from_str::<Trade>(&trade_json) {
                    Ok(trade) => Some(trade),
                    Err(_error) => None,
                },
            )
            .collect()
    }

    pub fn get_trades_result(&mut self) -> TradingResult {
        TradingResult::aggregate(self.get_all_trades())
    }

    pub fn clear_trades(&mut self) {
        self.trades.set_len(0).unwrap();
    }
}

#[derive(Debug)]
pub struct TradingResult {
    sold: f64,
    bought: f64,
}

impl TradingResult {
    pub fn aggregate(trades: Vec<Trade>) -> TradingResult {
        let (sold, bought) =
            trades
                .into_iter()
                .fold((0f64, 0f64), |acc, order| match order.side {
                    Side::Buy => (acc.0, acc.1 + order.amount),
                    Side::Sell => (acc.0 + order.amount, acc.1),
                });
        TradingResult { sold, bought }
    }
}

impl std::fmt::Display for TradingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Sold: {} | Bought: {} | Profit: {}",
            self.sold,
            self.bought,
            self.bought - self.sold
        )
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Trade {
    pub id: String,
    pub coins: Coins,
    pub side: Side,
    pub target: Target,
    pub price: f64,
    pub amount: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum Coins {
    TonUsdt,
}

impl From<trading_pair::Coins> for Coins {
    fn from(coins: trading_pair::Coins) -> Self {
        match coins {
            trading_pair::Coins::TonUsdt => Coins::TonUsdt,
        }
    }
}

impl From<Coins> for trading_pair::Coins {
    fn from(coins: Coins) -> Self {
        match coins {
            Coins::TonUsdt => trading_pair::Coins::TonUsdt,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum Side {
    Buy,
    Sell,
}

impl From<trading_pair::Side> for Side {
    fn from(side: trading_pair::Side) -> Self {
        match side {
            trading_pair::Side::Sell => Side::Sell,
            trading_pair::Side::Buy => Side::Buy,
        }
    }
}

impl From<Side> for trading_pair::Side {
    fn from(side: Side) -> Self {
        match side {
            Side::Sell => trading_pair::Side::Sell,
            Side::Buy => trading_pair::Side::Buy,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum Target {
    Market,
    Limit,
}

impl From<trade::Trade> for Trade {
    fn from(trade: trade::Trade) -> Trade {
        let (id, coins, side, target, price, amount) = match trade {
            trade::Trade::Market(result) => (
                result.id,
                result.trading_pair.coins,
                result.trading_pair.side, 
                Target::Market,
                result.price,
                result.amount
            ),
            trade::Trade::Limit(order) => (
                order.id,
                order.trading_pair.coins,
                order.trading_pair.side,
                Target::Limit,
                order.price,
                order.amount,
            ),
        };
        Trade {
            id, 
            coins: coins.into(),
            side: side.into(),
            target,
            price,
            amount,
        }
    }
}

impl From<Trade> for trade::Trade {
    fn from(trade: Trade) -> trade::Trade {
        let id = trade.id;
        let coins = trade.coins.into();
        let side = trade.side.into();
        let amount = trade.amount;
        let price = trade.price;
        match trade.target {
            Target::Market => trade::Trade::Market(TradeResult {
                id,
                trading_pair: trading_pair::TradingPair {
                    coins,
                    side,
                    target: trading_pair::Target::Market,
                },
                price,
                amount,
            }),
            Target::Limit => trade::Trade::Limit(OrderWithId {
                id,
                trading_pair: trading_pair::TradingPair {
                    coins,
                    side,
                    target: trading_pair::Target::Limit,
                },
                price,
                amount,
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let trade = agnostic::trade::Trade::Market(agnostic::trade::TradeResult {
            id: "1337".to_owned(),
            trading_pair: agnostic::trading_pair::TradingPair {
                coins: agnostic::trading_pair::Coins::TonUsdt,
                side: agnostic::trading_pair::Side::Sell,
                target: agnostic::trading_pair::Target::Market,
            },
            price: 33f64,
            amount: 100f64,
        });
        let mut bookkeeper = Bookkeeper::new().expect("Failed to create bookkeeper");
        bookkeeper.clear_trades();
        let orders = bookkeeper.get_all_trades();
        assert_eq!(orders.len(), 0, "Invalid length");
        bookkeeper.commit_trade(trade.clone());
        let orders = bookkeeper.get_all_trades();
        assert_eq!(orders.len(), 1, "Invalid length");
        bookkeeper.commit_trade(trade);
        let orders = bookkeeper.get_all_trades();
        assert_eq!(orders.len(), 2, "Invalid length");
    }
}
