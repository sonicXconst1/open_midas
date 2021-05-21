use crate::calculators::amount_calculator::Balance;
use crate::calculators::{AmountCalculator, ProfitCalculator};
use crate::filters::LowAmountFilter;
use agnostic::merchant::Merchant;
use agnostic::order::Order;
use agnostic::trade::Trade;
use agnostic::trading_pair::{Coins, TradingPair};
use agnostic::trading_pair::{Side, Target};
use std::collections::HashMap;

pub type Price = f64;
pub type Amount = f64;
pub type Storage = HashMap<Coins, Vec<Entry>>;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Entry {
    pub price: Price,
    pub amount: Amount,
}

impl Entry {
    pub fn incremented(&mut self, amount: f64) {
        self.amount += amount;
    }
}

pub struct Reseller<'a> {
    merchants: Vec<&'a dyn Merchant>,
    pub market_buy_storage: Storage,
    pub market_sell_storage: Storage,
    low_amount_filter: LowAmountFilter,
    amount_calculator: AmountCalculator,
    min_profit: f64,
}

impl<'a> Reseller<'a> {
    pub fn new(
        merchants: Vec<&'a dyn Merchant>,
        low_amount_filter: LowAmountFilter,
        amount_calculator: AmountCalculator,
        min_profit: f64,
    ) -> Reseller<'a> {
        Reseller {
            merchants,
            low_amount_filter,
            amount_calculator,
            market_buy_storage: Storage::new(),
            market_sell_storage: Storage::new(),
            min_profit,
        }
    }

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
        let target = Target::Market;
        for side in &[Side::Sell, Side::Buy] {
            let (storage, entry_side) = match side {
                Side::Sell => (&mut self.market_buy_storage, Side::Buy),
                Side::Buy => (&mut self.market_sell_storage, Side::Sell),
            };
            for (coins, entries) in storage.iter_mut() {
                let (entry_index, the_best_entry) =
                    match find_best_entry(&entries, entry_side) {
                        Some(entry) => entry,
                        None => continue,
                    };
                let trading_pair = TradingPair {
                    coins: coins.clone(),
                    target,
                    side: side.clone(),
                };
                let (the_best_order, merchant) = find_the_best_order(
                    the_best_entry,
                    &self.merchants,
                    trading_pair,
                    &self.amount_calculator,
                    &self.low_amount_filter,
                )
                .await?;
                let profit_calculator = ProfitCalculator::default();
                let (sell_price, buy_price) = match side.clone() {
                    Side::Sell => (the_best_order.price, the_best_entry.price),
                    Side::Buy => (the_best_entry.price, the_best_order.price),
                };
                match profit_calculator.evaluate(sell_price, buy_price) {
                    Some(profit) => {
                        if profit >= self.min_profit {
                            let trader = merchant.trader();
                            match trader.create_order(the_best_order.clone()).await {
                                Ok(trade) => {
                                    if the_best_entry.amount - trade.amount() <= 0.0 {
                                        entries.remove(entry_index);
                                    } else {
                                        let entry = entries.get_mut(entry_index).unwrap();
                                        entry.amount -= trade.amount()
                                    };
                                    self.accept_trade(trade.clone());
                                    return Ok(trade);
                                }
                                Err(error) => return Err(format!(
                                    "Failed to create an order {:#?}\n\t Error!: {:#?}",
                                    the_best_order,
                                    error)),
                            }
                        }
                    }
                    None => continue,
                }
            }
        }
        Err("No good orders to trade.".to_owned())
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

fn find_best_entry(entries: &[Entry], side: Side) -> Option<(usize, &Entry)> {
    match side {
        Side::Sell => entries
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.price.partial_cmp(&right.1.price).unwrap()),
        Side::Buy => entries
            .iter()
            .enumerate()
            .min_by(|left, right| left.1.price.partial_cmp(&right.1.price).unwrap()),
    }
}

async fn find_the_best_order<'a>(
    entry: &Entry,
    merchants: &[&'a dyn Merchant],
    pair: TradingPair,
    amount_calculator: &AmountCalculator,
    low_amount_filter: &LowAmountFilter,
) -> Result<(Order, &'a dyn Merchant), String> {
    let mut result = None;
    let mut the_best_merchant = None;
    let mut result_error = String::new();
    for merchant in merchants.iter() {
        let sniffer = merchant.sniffer();
        let orders = sniffer.all_the_best_orders(pair.clone(), 15).await?;
        let orders = low_amount_filter.filter(orders);
        let the_best_order = match orders.get(0) {
            Some(order) => order,
            None => {
                return Err(format!(
                    "Sniffer failed to sniff orders.\nPair: {:#?}\nOrders: {:#?}",
                    pair, orders
                ))
            }
        };
        let accountant = merchant.accountant();
        let currency = match accountant.ask(pair.coin_to_spend()).await {
            Ok(currency) => currency,
            Err(error) => {
                result_error.push_str(&error);
                continue;
            }
        };
        let currency_to_spend = agnostic::price::convert_to_base_coin_amount(
            pair.target.clone(),
            pair.side.clone(),
            &the_best_order.price.into(),
            currency.amount,
        );
        let balance = Balance {
            amount: currency_to_spend,
            fee: amount_calculator.fee,
        };
        let amount =
            match amount_calculator.evaluate(the_best_order.amount.min(entry.amount), &balance) {
                Some(amount) => amount.value(),
                None => continue,
            };
        match (pair.side.clone(), &mut result) {
            (_, None) => {
                result = Some(Order {
                    trading_pair: pair.clone(),
                    price: the_best_order.price,
                    amount,
                });
                the_best_merchant = Some(merchant);
            }
            (Side::Sell, Some(order)) => {
                if the_best_order.price > order.price {
                    order.price = the_best_order.price;
                    order.amount = amount;
                    the_best_merchant = Some(merchant);
                }
            }
            (Side::Buy, Some(order)) => {
                if the_best_order.price < order.price {
                    order.price = the_best_order.price;
                    order.amount = amount;
                    the_best_merchant = Some(merchant);
                }
            }
        }
    }
    match (result, the_best_merchant) {
        (Some(order), Some(merchant)) => Ok((order, *merchant)),
        _ => Err(format!("Failed to find the best order: {}", result_error)),
    }
}
