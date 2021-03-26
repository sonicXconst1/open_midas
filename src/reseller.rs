use agnostic::trade::{TradeResult, Trade};
use agnostic::trading_pair::Side;
use agnostic::order::OrderWithId;
use agnostic::merchant::Merchant;
use agnostic::trading_pair::Coins;
use std::sync::Arc;

pub type Price = f64;
pub type Amount = f64;

pub struct Entry {
    pub coins: Coins,
    pub price: Price,
    pub amount: Amount,
}

impl Entry {
    pub fn incremented(&mut self, amount: f64) {
        self.amount += amount;
    }
}

pub struct Reseller {
    // It's better to use HashSet<Coins, Vec<Entry>>
    market_buy_storage: Vec<Entry>,
    market_sell_storage: Vec<Entry>,
    merchant: Arc<dyn Merchant>,
}

impl Reseller {
    pub fn accept_trade(&mut self, trade: Trade) { 
        let coins = trade.trading_pair().coins;
        let price = trade.price();
        let amount = trade.amount();
        let storage: &mut Vec<_> = match trade {
            Trade::Market(trade_result) => {
                match trade_result.trading_pair.side {
                    Side::Sell => {
                        &mut self.market_sell_storage
                    },
                    Side::Buy => {
                        &mut self.market_buy_storage
                    }
                }
            },
            Trade::Limit(_order_with_id) => {
                unimplemented!("Limit orders are not supported.")
            },
        };
        accept_new_item(storage, coins, price, amount)
    }

    pub async fn iterate(&self) -> Trade {
        let accountant = self.merchant.accountant();
        let sniffer = self.merchant.sniffer();
        let trade = self.merchant.trader();
        // process sell side first
        unimplemented!()
    }
}

fn accept_new_item(
    storage: &mut Vec<Entry>,
    coins: Coins,
    new_price: Price,
    new_amount: Amount) {
    match storage.iter_mut().find(|entry| entry.coins == coins && entry.price == new_price) {
        Some(entry) => entry.incremented(new_amount),
        None => {
            storage.push(Entry {
                coins, 
                price: new_price,
                amount: new_amount,
            })
        }
    }
}
