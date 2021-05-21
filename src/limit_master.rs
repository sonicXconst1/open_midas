//! Limit Master
//!
//! There are two main stages: check current limit orders state & update current limit orders
//! state.
//!
//! Check: If order was updated - returns a Trade.
//!
//! Update: If there is a possibility to create better trade - do it. Or maybe it's better to
//! remove all current orders and simply place new ones?
//! Accepts price estimator. 
//! Price estimator calculate a price from orders & trading_pair.
//! Price estimator calculates the price for good order without any respect to curreny balance.
//!
//! Result of the iteration is a vector of orders.
//! Then reseller accepts trade and performs an iteration.
//!
use agnostic::merchant::Merchant;
use agnostic::trade::Trade;
use agnostic::trading_pair::{Coins, Side, Target, TradingPair};
use agnostic::order::{Order, OrderWithId};
use crate::calculators::AmountCalculator;
use crate::calculators::amount_calculator::Balance;
use crate::calculators::price_calculator::PriceCalculator;

pub type MerchantId = usize;

#[derive(Clone, Debug)]
pub struct OrderEntity<TOrder> {
    pub merchant_id: MerchantId,
    pub order: TOrder,
}

impl<TOrder> OrderEntity<TOrder> {
    pub fn new(merchant_id: MerchantId, order: TOrder) -> Self {
        OrderEntity {
            merchant_id,
            order,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OrdersStorage<TOrder> {
    pub coins: Coins,
    sell_stock: Vec<OrderEntity<TOrder>>,
    buy_stock: Vec<OrderEntity<TOrder>>,
}

impl<TOrder> OrdersStorage<TOrder> {
    pub fn get_stock(&self, target: Target, side: Side) -> &[OrderEntity<TOrder>] {
        match (target, side) {
            (Target::Limit, Side::Buy) => &self.buy_stock[..],
            (Target::Limit, Side::Sell) => &self.sell_stock[..],
            (Target::Market, Side::Buy) => &self.sell_stock[..],
            (Target::Market, Side::Sell) => &self.buy_stock[..],
        }
    }
}

pub struct MerchantIdManager<'a> {
    merchants: &'a [&'a dyn Merchant],
}

impl<'a> MerchantIdManager<'a> {
    pub fn new(merchants: &'a [&'a dyn Merchant]) -> Self {
        MerchantIdManager {
            merchants
        }
    }

    pub fn get_mercahnt_id(&self, merchant: &dyn Merchant) -> Option<MerchantId> {
        self.merchants
            .iter()
            .enumerate()
            .find(|(_index, m)| std::ptr::eq(**m, merchant))
            .map(|(index, _merchant)| index)
    }

    pub fn get_merchant(&self, id: MerchantId) -> Option<&dyn Merchant> {
        self.merchants.get(id).map(|value| *value)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, &dyn Merchant> {
        self.merchants.iter()
    }
}

pub struct LimitMaster<'a> {
    coins: Coins,
    merchants_manager: MerchantIdManager<'a>,
    my_orders_last_state: OrdersStorage<OrderWithId>,
    price_calculator: PriceCalculator,
    amount_calculator: AmountCalculator,
}

impl<'a> LimitMaster<'a> {
    pub fn new(
        coins: Coins,
        merchants_manager: MerchantIdManager<'a>,
        price_calculator: PriceCalculator,
        amount_calculator: AmountCalculator
    ) -> Self {
        LimitMaster {
            coins: coins.clone(),
            merchants_manager,
            price_calculator,
            amount_calculator,
            my_orders_last_state: OrdersStorage {
                coins,
                sell_stock: Vec::with_capacity(16),
                buy_stock: Vec::with_capacity(16),
            }
        }
    }

    pub async fn check_current_orders(&mut self) -> Result<Vec<Trade>, String> {
        let mut performed_trades = Vec::new();
        let my_current_orders = self.accumulate_my_current_order().await;
        for last_order in self.my_orders_last_state.buy_stock.iter_mut() {
            for current_order in my_current_orders.buy_stock.iter() {
                if last_order.merchant_id == current_order.merchant_id 
                    && last_order.order.id == current_order.order.id {
                    if last_order.order.amount > current_order.order.amount {
                        let mut performed_order = last_order.order.clone();
                        performed_order.amount -= current_order.order.amount;
                        last_order.order.amount = current_order.order.amount;
                        performed_trades.push(Trade::Limit(performed_order));
                    } 
                }
            }
        }
        Ok(performed_trades)
    }

    pub async fn update_orders(&mut self) -> Result<(), String> {
        self.delete_all_my_orders().await?;
        let current_orders_storage = self.accumulate_merchants_infomration().await;
        self.update_orders_on_side(Side::Buy, &current_orders_storage).await.unwrap();
        self.update_orders_on_side(Side::Sell, &current_orders_storage).await
    }

    async fn update_orders_on_side(
        &mut self,
        side: Side,
        current_orders_storage: &OrdersStorage<Order>) -> Result<(), String> {
        let coins = self.coins.clone();
        let market_stock: Vec<_> = current_orders_storage
            .get_stock(Target::Market, side)
            .iter()
            .filter(|entity| entity.order.amount > 100.0)
            .collect();
        let best_stock_order = market_stock.get(0).unwrap();
        let market_trading_pair = TradingPair {
            coins,
            side,
            target: Target::Market,
        };
        let price_for_limit_order = match side {
            Side::Buy => self.price_calculator.high(best_stock_order.order.price),
            Side::Sell => self.price_calculator.low(best_stock_order.order.price),
        };
        for merchant in self.merchants_manager.iter() {
            let accountant = merchant.accountant();
            let balance = accountant.ask(market_trading_pair.coin_to_spend()).await?;
            let balance = Balance {
                amount: balance.amount,
                fee: self.amount_calculator.fee,
            };
            let limit_order_amount = self.amount_calculator.evaluate(
                best_stock_order.order.amount,
                &balance).unwrap();
            let trader = merchant.trader();
            match trader.create_order(Order {
                trading_pair: TradingPair {
                    coins,
                    side,
                    target: Target::Limit,
                },
                price: price_for_limit_order,
                amount: limit_order_amount.value()
            }).await {
                Ok(Trade::Limit(order)) => {
                    let merchant_id = self.merchants_manager.get_mercahnt_id(*merchant).unwrap();
                    let stock = match side {
                        Side::Buy => &mut self.my_orders_last_state.sell_stock,
                        Side::Sell => &mut self.my_orders_last_state.buy_stock,
                    };
                    stock.push(OrderEntity {
                        merchant_id,
                        order,
                    });
                },
                _ => panic!("Failed to create order"),
            };
        }
        Ok(())
    }

    async fn delete_all_my_orders(&mut self) -> Result<(), String> {
        for merchant in self.merchants_manager.iter() {
            let sniffer = merchant.sniffer();
            let trader = merchant.trader();
            for side in &[Side::Sell, Side::Buy] {
                let trading_pair = TradingPair {
                    coins: self.coins.clone(),
                    side: *side,
                    target: Target::Limit,
                };
                let my_orders = sniffer.get_my_orders(trading_pair).await?;
                for order in my_orders.iter() {
                    trader.delete_order(order.id.as_ref()).await?;
                }
            }
        }
        Ok(())
    }

    async fn accumulate_merchants_infomration(&self) -> OrdersStorage<Order> {
        self.accumulate(|merchant, trading_pair| {
            let sniffer = merchant.sniffer();
            let future = async move {
                sniffer.all_the_best_orders(trading_pair, 15).await.unwrap()
            };
            Box::pin(future)
        }).await
    }

    async fn accumulate_my_current_order(&self) -> OrdersStorage<OrderWithId> {
        self.accumulate(|merchant, trading_pair| {
            let sniffer = merchant.sniffer();
            let future = async move {
                sniffer.get_my_orders(trading_pair).await.unwrap()
            };
            Box::pin(future)
        }).await
    }

    async fn accumulate<TOutput: std::iter::IntoIterator>(
        &self,
        sniff_callback: impl Fn(&dyn Merchant, TradingPair) -> std::pin::Pin<Box<dyn futures::Future<Output = TOutput>>>
    ) -> OrdersStorage<TOutput::Item> {
        let coins = self.coins.clone();
        let mut sell_orders_collection = Vec::new();
        let mut buy_orders_collection = Vec::new();
        for merchant in self.merchants_manager.iter() {
            let trading_pair = TradingPair {
                coins,
                side: Side::Sell,
                target: Target::Limit,
            };
            sniff_callback(*merchant, trading_pair).await
                .into_iter()
                .for_each(|order| sell_orders_collection.push(OrderEntity::new(
                            self.merchants_manager.get_mercahnt_id(*merchant).unwrap(),
                            order)));
        };
        for merchant in self.merchants_manager.iter() {
            let trading_pair = TradingPair {
                coins,
                side: Side::Buy,
                target: Target::Limit,
            };
            sniff_callback(*merchant, trading_pair).await
                .into_iter()
                .for_each(|order| buy_orders_collection.push(OrderEntity::new(
                            self.merchants_manager.get_mercahnt_id(*merchant).unwrap(),
                            order)));
        };
        OrdersStorage {
            coins,
            buy_stock: buy_orders_collection,
            sell_stock: sell_orders_collection,
        }
    }
}
