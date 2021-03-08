use std::io::Write;
use std::io::Read;
use agnostic::trading_pair;
use agnostic::order;

pub struct Bookkeeper {
    trades: std::fs::File,
}

impl Bookkeeper {
    const DEAFULT_TRADES_PATH: &'static str = "bookkeeper/trades.csv";
    const SPLITTER: char = '|';

    pub fn new() -> std::io::Result<Bookkeeper> {
        let trades_path = std::path::Path::new(Self::DEAFULT_TRADES_PATH);
        let trades = if trades_path.exists() {
            std::fs::File::open(trades_path)?
        } else {
            std::fs::File::create(trades_path)?
        };
        Ok(Bookkeeper {
            trades,
        })
    }

    pub fn commit_trade(&mut self, order: &order::Order) {
        let order: Order = order.into();
        let mut order = serde_json::to_string(&order).expect("Serialization error");
        order.push(Self::SPLITTER);
        self.trades.write_all(order.as_bytes()).expect("Failed to commit order");
    }

    pub fn get_all_trades(&mut self) -> Vec<Order> {
        let mut result = Vec::with_capacity(100);
        self.trades.read_to_end(&mut result).expect("Failed to read trades.");
        String::from_utf8(result)
            .expect("Invlaid UTF-8 string.")
            .split(Self::SPLITTER)
            .map(|order_json| serde_json::from_str::<Order>(&order_json)
                 .expect("Deserialization error")
                 .into())
            .collect()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Order {
    pub coins: Coins,
    pub side: Side,
    pub target: Target,
    pub price: f64,
    pub amount: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum Coins {
    TonUsdt,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
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

impl From<Order> for order::Order {
    fn from(order: Order) -> order::Order {
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
