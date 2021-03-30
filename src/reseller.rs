use agnostic::trade::{TradeResult, Trade};
use agnostic::order::{OrderWithId, Order};
use agnostic::merchant::Merchant;
use agnostic::trading_pair::{Side, Target};
use agnostic::trading_pair::{Coins, TradingPair};
use std::collections::HashMap;
use crate::calculators::{AmountCalculator, ProfitCalculator};
use crate::calculators::amount_calculator::Balance;
use std::sync::Arc;

pub type Price = f64;
pub type Amount = f64;
pub type Storage = HashMap<Coins, Vec<Entry>>;

pub struct Entry {
    pub price: Price,
    pub amount: Amount,
}

impl Entry {
    pub fn incremented(&mut self, amount: f64) {
        self.amount += amount;
    }
}

pub struct Reseller {
    market_buy_storage: Storage,
    market_sell_storage: Storage,
    merchant: Arc<dyn Merchant>,
}

impl Reseller {
    pub fn accept_trade(&mut self, trade: Trade) { 
        let coins = trade.trading_pair().coins;
        let price = trade.price();
        let amount = trade.amount();
        let storage: &mut Storage = match trade {
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
        accept_new_item(storage, &coins, price, amount)
    }

    pub async fn iterate(&mut self) -> Result<Trade, String> {
        let accountant = self.merchant.accountant();
        let sniffer = self.merchant.sniffer();
        let trader = self.merchant.trader();
        for (coins, entries) in self.market_sell_storage.iter_mut() {
            let trading_pair = TradingPair {
                coins: coins.clone(),
                target: Target::Market,
                side: Side::Buy,
            };
            let currency = accountant.ask(trading_pair.coin_to_spend()).await?;
            let orders = sniffer.all_the_best_orders(trading_pair.clone(), 5).await?;
            let the_best_order = match orders.get(0) {
                Some(order) => order,
                None => return Err(format!(
                    "Sniffer failed to sniff orders.\nPair: {:#?}\nOrders: {:#?}",
                    trading_pair,
                    orders)),
            };
            let the_best_entry = match entries.iter()
                .find(|entry| entry.price > the_best_order.price) {
                Some(entry) => entry,
                None => return Err("Failed to find entry with good price".to_owned()),
            };
            let currency_to_spend = agnostic::price::convert_to_base_coin_amount(
                trading_pair.target,
                trading_pair.side,
                &the_best_order.price.into(),
                currency.amount);
            let amount_calculator = AmountCalculator::new(0.1, 0.01)
                .expect("Invalid fee");
            let amount = match amount_calculator.calculate_from_one_order(
                the_best_order.amount.min(the_best_entry.amount),
                Balance {
                    amount: currency_to_spend,
                    fee: amount_calculator.fee,
                }) {
                Some(amount) => amount.value(),
                None => return Err("Failed to calculate amount".to_owned()),
            };
            match trader.create_order(Order {
                trading_pair: TradingPair {
                    coins: coins.clone(),
                    side: Side::Buy,
                    target: Target::Market,
                },
                price: the_best_order.price,
                amount,
            }).await {
                Ok(trade) => return Ok(trade),
                Err(_) => (),
            }
        }
        // process sell side first
        unimplemented!()
    }
}

fn accept_new_item(
    storage: &mut Storage,
    coins: &Coins,
    new_price: Price,
    new_amount: Amount) {
    let entries = match storage.get_mut(coins) {
        Some(entries) => entries,
        None => {
            storage.insert(coins.clone(), Vec::new());
            storage.get_mut(coins).unwrap()
        },
    };
    match entries.iter_mut().find(|entry| entry.price == new_price) {
        Some(entry) => entry.incremented(new_amount),
        None => {
            entries.push(Entry {
                price: new_price,
                amount: new_amount,
            })
        }
    }
}
