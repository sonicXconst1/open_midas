use agnostic::order;
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

    pub fn commit_trade(&mut self, order: &order::Order) {
        let order: Order = order.into();
        let mut order = serde_json::to_string(&order).expect("Serialization error");
        order.push(Self::SPLITTER);
        self.trades
            .seek(std::io::SeekFrom::End(0))
            .expect("Failed to seek to end");
        self.trades
            .write_all(order.as_bytes())
            .expect("Failed to commit order");
    }

    pub fn get_all_trades(&mut self) -> Vec<Order> {
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
                |order_json| match serde_json::from_str::<Order>(&order_json) {
                    Ok(order) => Some(order.into()),
                    Err(_error) => None,
                },
            )
            .collect()
    }

    pub fn get_trades_result(&mut self) -> TradingResult {
        self.get_all_trades().into()
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
    pub fn aggregate(trades: Vec<Order>) -> TradingResult {
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
pub struct Order {
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum Target {
    Market,
    Limit,
}

impl From<&order::Order> for Order {
    fn from(order: &order::Order) -> Order {
        Order {
            coins: match order.trading_pair.coins {
                trading_pair::Coins::TonUsdt => Coins::TonUsdt,
            },
            side: match order.trading_pair.side {
                trading_pair::Side::Buy => Side::Buy,
                trading_pair::Side::Sell => Side::Sell,
            },
            target: match order.trading_pair.target {
                trading_pair::Target::Market => Target::Market,
                trading_pair::Target::Limit => Target::Limit,
            },
            price: order.price,
            amount: order.amount,
        }
    }
}

impl From<&Order> for order::Order {
    fn from(order: &Order) -> order::Order {
        order::Order {
            trading_pair: trading_pair::TradingPair {
                coins: match order.coins {
                    Coins::TonUsdt => trading_pair::Coins::TonUsdt,
                },
                side: match order.side {
                    Side::Sell => trading_pair::Side::Sell,
                    Side::Buy => trading_pair::Side::Buy,
                },
                target: match order.target {
                    Target::Market => trading_pair::Target::Market,
                    Target::Limit => trading_pair::Target::Limit,
                },
            },
            price: order.price,
            amount: order.amount,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let order = agnostic::order::Order {
            trading_pair: agnostic::trading_pair::TradingPair {
                coins: agnostic::trading_pair::Coins::TonUsdt,
                side: agnostic::trading_pair::Side::Sell,
                target: agnostic::trading_pair::Target::Market,
            },
            price: 33f64,
            amount: 100f64,
        };
        let mut bookkeeper = Bookkeeper::new().expect("Failed to create bookkeeper");
        bookkeeper.clear_trades();
        let orders = bookkeeper.get_all_trades();
        assert_eq!(orders.len(), 0, "Invalid length");
        bookkeeper.commit_trade(&order);
        let orders = bookkeeper.get_all_trades();
        assert_eq!(orders.len(), 1, "Invalid length");
        bookkeeper.commit_trade(&order);
        let orders = bookkeeper.get_all_trades();
        assert_eq!(orders.len(), 2, "Invalid length");
    }
}
