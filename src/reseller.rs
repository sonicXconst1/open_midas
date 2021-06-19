use crate::calculators::amount_calculator::Balance;
use crate::calculators::{AmountCalculator, ProfitCalculator};
use crate::filters::LowAmountFilter;
use agnostic::merchant::Merchant;
use agnostic::order::Order;
use agnostic::trade::Trade;
use agnostic::trading_pair::{Coin, Coins, TradingPair};
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
    pub buy_storage: Storage,
    pub sell_storage: Storage,
    low_amount_filter: LowAmountFilter,
    amount_calculator: AmountCalculator,
    min_profit: f64,
    auto_accept: bool,
}

impl<'a> Reseller<'a> {
    pub fn new(
        buy_storage: Storage,
        sell_storage: Storage,
        merchants: Vec<&'a dyn Merchant>,
        low_amount_filter: LowAmountFilter,
        amount_calculator: AmountCalculator,
        min_profit: f64,
        auto_accept: bool,
    ) -> Reseller<'a> {
        Reseller {
            merchants,
            low_amount_filter,
            amount_calculator,
            buy_storage,
            sell_storage,
            min_profit,
            auto_accept,
        }
    }

    pub fn accept_trade(&mut self, trade: Trade) {
        let coins = trade.trading_pair().coins;
        let price = trade.price();
        let amount = trade.amount();
        let storage: &mut Storage = match trade.trading_pair().side {
            Side::Sell => &mut self.sell_storage,
            Side::Buy => &mut self.buy_storage,
        };
        accept_new_item(storage, &coins, price, amount)
    }

    pub async fn iterate(&mut self) -> Result<Option<Trade>, String> {
        let target = Target::Market;
        for iteration_side in &[Side::Sell, Side::Buy] {
            let (storage, entry_side) = match iteration_side {
                Side::Sell => (&mut self.buy_storage, Side::Buy),
                Side::Buy => (&mut self.sell_storage, Side::Sell),
            };
            log::debug!("Storage {} with {} entries", entry_side, storage.len());
            for (coins, entries) in storage.iter_mut() {
                let (entry_index, the_best_entry) =
                    match find_best_entry(&entries, entry_side) {
                        Some(entry) => entry,
                        None => continue,
                    };
                log::debug!(
                    "Best entry: Price {:<8.3} Amount {:^8.3}", 
                    the_best_entry.price,
                    the_best_entry.amount);
                let trading_pair = TradingPair {
                    coins: coins.clone(),
                    target,
                    side: iteration_side.clone(),
                };
                let (the_best_order, merchant) = match find_the_best_order(
                    the_best_entry,
                    &self.merchants,
                    trading_pair,
                    &self.amount_calculator,
                    &self.low_amount_filter,
                )
                .await
                {
                    Ok(find_result) => find_result,
                    Err(error) => match error {
                        FindError::NoProfit => continue,
                        other => {
                            return Err(format!("Find error: {}", other));
                        }
                    },
                };
                log::debug!(
                    "The best order: Side {:<8} Price {:^10.3} Amount {}",
                    the_best_order.trading_pair.side,
                    the_best_order.price,
                    the_best_order.amount);
                let profit_calculator = ProfitCalculator::default();
                let (sell_price, buy_price) = match iteration_side.clone() {
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
                                    if self.auto_accept {
                                        self.accept_trade(trade.clone());
                                    }
                                    return Ok(Some(trade));
                                }
                                Err(error) => return Err(format!(
                                    "Failed to create an order {:#?}\n\t Error!: {:#?}",
                                    the_best_order, error
                                )),
                            }
                        }
                    }
                    None => continue,
                }
            }
        }
        Ok(None)
    }
}

fn accept_new_item(
    storage: &mut Storage,
    coins: &Coins,
    new_price: Price,
    new_amount: Amount,
) {
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
            .min_by(|left, right| left.1.price.partial_cmp(&right.1.price).unwrap()),
        Side::Buy => entries
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.price.partial_cmp(&right.1.price).unwrap()),
    }
}

pub enum FindError {
    NoProfit,
    EmptyStock,
    SnifferError(String),
    AccountantError((Coin, String)),
}

impl std::fmt::Display for FindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FindError::NoProfit => write!(f, "No Profit"),
            FindError::EmptyStock => write!(f, "Empty Stock"),
            FindError::SnifferError(error) => write!(f, "{}", error),
            FindError::AccountantError((_coin, error)) => write!(f, "{}", error),
        }
    }
}

async fn find_the_best_order<'a>(
    entry: &Entry,
    merchants: &[&'a dyn Merchant],
    pair: TradingPair,
    amount_calculator: &AmountCalculator,
    low_amount_filter: &LowAmountFilter,
) -> Result<(Order, &'a dyn Merchant), FindError> {
    let mut result = None;
    let mut the_best_merchant = None;
    for merchant in merchants.iter() {
        let sniffer = merchant.sniffer();
        let orders = match sniffer.all_the_best_orders(pair.clone(), 15).await {
            Ok(orders) => orders,
            Err(error) => {
                return Err(FindError::SnifferError(error));
            }
        };
        let orders = low_amount_filter.filter(orders);
        let the_best_order = match orders.get(0) {
            Some(order) => order,
            None => {
                return Err(FindError::EmptyStock);
            }
        };
        let accountant = merchant.accountant();
        let currency = match accountant.ask(pair.coin_to_spend()).await {
            Ok(currency) => currency,
            Err(error) => {
                return Err(FindError::AccountantError((pair.coin_to_spend(), error)));
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
        let amount = match amount_calculator
            .evaluate(the_best_order.amount.min(entry.amount), &balance)
        {
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
        _ => Err(FindError::NoProfit),
    }
}
