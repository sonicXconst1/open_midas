use crate::calculators::amount_calculator::Balance;
use crate::calculators::{AmountCalculator, ProfitCalculator};
use crate::filters::LowAmountFilter;
use agnostic::merchant::Merchant;
use agnostic::order::{Order, OrderWithId};
use agnostic::trade::{Trade, TradeResult};
use agnostic::trading_pair::{Coins, TradingPair};
use agnostic::trading_pair::{Side, Target};
use std::collections::HashMap;
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
    low_amount_filter: LowAmountFilter,
}

impl Reseller {
    pub fn accept_trade(&mut self, trade: Trade) {
        let coins = trade.trading_pair().coins;
        let price = trade.price();
        let amount = trade.amount();
        let storage: &mut Storage = match trade {
            Trade::Market(trade_result) => match trade_result.trading_pair.side {
                Side::Sell => &mut self.market_sell_storage,
                Side::Buy => &mut self.market_buy_storage,
            },
            Trade::Limit(_order_with_id) => {
                unimplemented!("Limit orders are not supported.")
            }
        };
        accept_new_item(storage, &coins, price, amount)
    }

    pub async fn iterate(&mut self) -> Result<Trade, String> {
        match self.iterate_market(Side::Buy).await? {
            Some(trade) => Ok(trade),
            None => match self.iterate_market(Side::Sell).await? {
                Some(trade) => Ok(trade),
                None => Err("No good orders to trade.".to_owned()),
            },
        }
    }

    async fn iterate_market(&mut self, market_storage_side: Side) -> Result<Option<Trade>, String> {
        let accountant = self.merchant.accountant();
        let sniffer = self.merchant.sniffer();
        let trader = self.merchant.trader();
        let storage = match market_storage_side {
            Side::Buy => &mut self.market_buy_storage,
            Side::Sell => &mut self.market_sell_storage,
        };
        for (coins, entries) in storage.iter_mut() {
            let trading_pair = TradingPair {
                coins: coins.clone(),
                target: Target::Market,
                side: market_storage_side.clone(),
            };
            let trading_pair = trading_pair.reversed_side();
            let currency = accountant.ask(trading_pair.coin_to_spend()).await?;
            let orders = sniffer.all_the_best_orders(trading_pair.clone(), 5).await?;
            let orders = self.low_amount_filter.filter(orders);
            let the_best_order = match orders.get(0) {
                Some(order) => order,
                None => {
                    return Err(format!(
                        "Sniffer failed to sniff orders.\nPair: {:#?}\nOrders: {:#?}",
                        trading_pair, orders
                    ))
                }
            };
            let profit_calculator = ProfitCalculator::default();
            let the_best_entry = match entries.iter().find(|entry| {
                let (sell_price, buy_price) = match market_storage_side {
                    Side::Buy => (the_best_order.price, entry.price),
                    Side::Sell => (entry.price, the_best_order.price),
                };
                profit_calculator
                    .evaluate(sell_price, buy_price)
                    .map_or(false, |profit| profit >= 0.03)
            }) {
                Some(entry) => entry,
                None => return Err("Failed to find entry with good price".to_owned()),
            };
            let currency_to_spend = agnostic::price::convert_to_base_coin_amount(
                trading_pair.target.clone(),
                trading_pair.side.clone(),
                &the_best_order.price.into(),
                currency.amount,
            );
            let amount_calculator = AmountCalculator::new(0.1, 0.01).expect("Invalid fee");
            let amount = match amount_calculator.evaluate(
                the_best_order.amount.min(the_best_entry.amount),
                Balance {
                    amount: currency_to_spend,
                    fee: amount_calculator.fee,
                },
            ) {
                Some(amount) => amount.value(),
                None => return Err("Failed to calculate amount".to_owned()),
            };
            match trader
                .create_order(Order {
                    trading_pair: trading_pair.clone(),
                    price: the_best_order.price,
                    amount,
                })
                .await
            {
                Ok(trade) => return Ok(Some(trade)),
                Err(_) => (),
            }
        }
        Ok(None)
    }
}

fn accept_new_item(storage: &mut Storage, coins: &Coins, new_price: Price, new_amount: Amount) {
    let entries = match storage.get_mut(coins) {
        Some(entries) => entries,
        None => {
            storage.insert(coins.clone(), Vec::new());
            storage.get_mut(coins).unwrap()
        }
    };
    match entries.iter_mut().find(|entry| entry.price == new_price) {
        Some(entry) => entry.incremented(new_amount),
        None => entries.push(Entry {
            price: new_price,
            amount: new_amount,
        }),
    }
}
