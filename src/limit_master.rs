//! Limit Master
//!
//! There are two main stages: check current limit orders state & update current limit orders
//! state.
//!
//! TODO: Allow Limit Master to load last OrdersStorage. It will be requiered for deserialization.
//! Now it is requiered for testing check method wihtout calling an update.
use crate::calculators::amount_calculator::Balance;
use crate::calculators::price_calculator::PriceCalculator;
use crate::calculators::AmountCalculator;
use crate::deleter::Deleter;
use agnostic::merchant::Merchant;
use agnostic::order::{Order, OrderWithId};
use agnostic::trade::Trade;
use agnostic::trading_pair::{Coins, Side, Target, TradingPair};

pub type MerchantId = &'static str;

#[derive(Clone, Debug)]
pub struct OrderEntity<TOrder> {
    pub merchant_id: MerchantId,
    pub order: TOrder,
}

pub fn entity_to_string(entity: OrderEntity<OrderWithId>) -> String {
    format!(
        "{:8} {:10} {:10} price {:11.5} amount {:11.5}",
        entity.merchant_id,
        entity.order.trading_pair.side,
        entity.order.trading_pair.target,
        entity.order.price,
        entity.order.amount)
}

impl<TOrder> OrderEntity<TOrder> {
    pub fn new(merchant_id: MerchantId, order: TOrder) -> Self {
        OrderEntity { merchant_id, order }
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

    pub fn clear(&mut self) {
        self.sell_stock.clear();
        self.buy_stock.clear();
    }
}

pub struct MerchantIdManager<'a> {
    merchants: &'a [&'a dyn Merchant],
}

impl<'a> MerchantIdManager<'a> {
    pub fn new(merchants: &'a [&'a dyn Merchant]) -> Self {
        MerchantIdManager { merchants }
    }

    pub fn get_mercahnt_id(&self, merchant: &dyn Merchant) -> Option<MerchantId> {
        self.merchants
            .iter()
            .find(|m| std::ptr::eq(**m, merchant))
            .map(|merchant| merchant.id())
    }

    pub fn get_merchant(&self, id: MerchantId) -> Option<&dyn Merchant> {
        self.merchants.iter()
            .find(|merchant| merchant.id() == id)
            .map(|m| *m)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, &dyn Merchant> {
        self.merchants.iter()
    }
}

#[derive(Clone, Debug)]
pub struct Update {
    pub sell: Vec<OrderEntity<OrderWithId>>,
    pub buy: Vec<OrderEntity<OrderWithId>>,
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
        amount_calculator: AmountCalculator,
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
            },
        }
    }

    pub async fn check_current_orders(&mut self) -> Result<Vec<Trade>, String> {
        let my_current_orders = self.accumulate_my_current_order().await;
        let performed_trades = self.my_orders_last_state.buy_stock.iter_mut().fold(
            Vec::with_capacity(16),
            |mut acc, last_order| {
                my_current_orders
                    .buy_stock
                    .iter()
                    .find(|item| {
                        item.merchant_id == last_order.merchant_id
                            && item.order.id == last_order.order.id
                    })
                    .map_or_else(
                        || Some(Trade::Limit(last_order.order.clone())),
                        |current_order| {
                            if current_order.order.amount < last_order.order.amount {
                                let mut performed_order = last_order.order.clone();
                                performed_order.amount -= current_order.order.amount;
                                Some(Trade::Limit(performed_order))
                            } else {
                                None
                            }
                        },
                    )
                    .and_then(|trade| {
                        last_order.order.amount -= trade.amount();
                        Some(acc.push(trade))
                    });
                acc
            },
        );
        Ok(self.my_orders_last_state.sell_stock.iter_mut().fold(
            performed_trades,
            |mut acc, last_order| {
                my_current_orders
                    .sell_stock
                    .iter()
                    .find(|item| {
                        item.merchant_id == last_order.merchant_id
                            && item.order.id == last_order.order.id
                    })
                    .map_or_else(
                        || Some(Trade::Limit(last_order.order.clone())),
                        |current_order| {
                            if current_order.order.amount < last_order.order.amount {
                                let mut performed_order = last_order.order.clone();
                                performed_order.amount -= current_order.order.amount;
                                Some(Trade::Limit(performed_order))
                            } else {
                                None
                            }
                        },
                    )
                    .and_then(|trade| {
                        last_order.order.amount -= trade.amount();
                        Some(acc.push(trade))
                    });
                acc
            },
        ))
    }

    pub async fn update_orders(&mut self) -> Result<Update, String> {
        self.delete_all_my_orders().await?;
        let current_orders_storage = self.accumulate_merchants_infomration().await;
        let buy = self
            .update_orders_on_side(Side::Buy, &current_orders_storage)
            .await?;
        let sell = self
            .update_orders_on_side(Side::Sell, &current_orders_storage)
            .await?;
        Ok(Update { buy, sell })
    }

    async fn update_orders_on_side(
        &mut self,
        side: Side,
        current_orders_storage: &OrdersStorage<Order>,
    ) -> Result<Vec<OrderEntity<OrderWithId>>, String> {
        let coins = self.coins.clone();
        let min_amount = self.amount_calculator.min_amount_threshold;
        let market_stock: Vec<_> = current_orders_storage
            .get_stock(Target::Market, side)
            .iter()
            .filter(|entity| entity.order.amount > min_amount)
            .collect();
        let best_stock_order = match side {
            Side::Buy => market_stock.iter().min_by(|left, right| left.order.price.partial_cmp(&right.order.price).unwrap()),
            Side::Sell => market_stock.iter().max_by(|left, right| left.order.price.partial_cmp(&right.order.price).unwrap()),
        };
        if best_stock_order.is_none() {
            return Ok(Vec::new());
        }
        let best_stock_order = best_stock_order.unwrap();
        let market_trading_pair = TradingPair {
            coins,
            side,
            target: Target::Market,
        };
        let price_for_limit_order = match side {
            Side::Buy => self.price_calculator.low(best_stock_order.order.price),
            Side::Sell => self.price_calculator.high(best_stock_order.order.price),
        };
        let mut orders = Vec::with_capacity(10);
        for merchant in self.merchants_manager.iter() {
            let accountant = merchant.accountant();
            let balance = accountant.ask(market_trading_pair.coin_to_spend()).await?;
            let balance = Balance {
                amount: balance.amount,
                fee: self.amount_calculator.fee,
            };
            let limit_order_amount = match self
                .amount_calculator
                .evaluate(best_stock_order.order.amount, &balance) {
                Some(result) => result,
                None => return Ok(orders),
            };
            let trader = merchant.trader();
            match trader
                .create_order(Order {
                    trading_pair: TradingPair {
                        coins,
                        side,
                        target: Target::Limit,
                    },
                    price: price_for_limit_order,
                    amount: limit_order_amount.value(),
                })
                .await
            {
                Ok(Trade::Limit(order)) => {
                    let merchant_id = self.merchants_manager.get_mercahnt_id(*merchant).unwrap();
                    let stock = match side {
                        Side::Buy => &mut self.my_orders_last_state.sell_stock,
                        Side::Sell => &mut self.my_orders_last_state.buy_stock,
                    };
                    let entity = OrderEntity { merchant_id, order };
                    orders.push(entity.clone());
                    stock.push(entity)
                }
                _ => panic!("Failed to create order"),
            };
        }
        println!("{:#?}", orders);
        Ok(orders)
    }

    pub async fn delete_all_my_orders(&mut self) -> Result<(), String> {
        self.my_orders_last_state.clear();
        Deleter::default().delete_all(
            self.merchants_manager.iter().as_slice(),
            self.coins.clone(),
        ).await
    }

    async fn accumulate_merchants_infomration(&self) -> OrdersStorage<Order> {
        self.accumulate(|merchant, trading_pair| {
            let sniffer = merchant.sniffer();
            let future =
                async move { sniffer.all_the_best_orders(trading_pair, 15).await.unwrap() };
            Box::pin(future)
        })
        .await
    }

    async fn accumulate_my_current_order(&self) -> OrdersStorage<OrderWithId> {
        self.accumulate(|merchant, trading_pair| {
            let sniffer = merchant.sniffer();
            let future = async move { sniffer.get_my_orders(trading_pair).await.unwrap() };
            Box::pin(future)
        })
        .await
    }

    async fn accumulate<TOutput: std::iter::IntoIterator>(
        &self,
        sniff_callback: impl Fn(
            &dyn Merchant,
            TradingPair,
        ) -> std::pin::Pin<Box<dyn futures::Future<Output = TOutput>>>,
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
            sniff_callback(*merchant, trading_pair)
                .await
                .into_iter()
                .for_each(|order| {
                    sell_orders_collection.push(OrderEntity::new(
                        self.merchants_manager.get_mercahnt_id(*merchant).unwrap(),
                        order,
                    ))
                });
        }
        for merchant in self.merchants_manager.iter() {
            let trading_pair = TradingPair {
                coins,
                side: Side::Buy,
                target: Target::Limit,
            };
            sniff_callback(*merchant, trading_pair)
                .await
                .into_iter()
                .for_each(|order| {
                    buy_orders_collection.push(OrderEntity::new(
                        self.merchants_manager.get_mercahnt_id(*merchant).unwrap(),
                        order,
                    ))
                });
        }
        OrdersStorage {
            coins,
            buy_stock: buy_orders_collection,
            sell_stock: sell_orders_collection,
        }
    }
}
