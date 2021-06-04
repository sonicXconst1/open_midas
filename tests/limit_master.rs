use agnostic::{
    merchant::Merchant,
    trading_pair::{Coins, Side, Target, TradingPair},
    order::OrderWithId,
    trade::Trade,
    market::{Accountant, Sniffer},
};
use agnostic_test::{
    merchant::Merchant as MerchantTest,
    trader::{TradesLogger, Trader as TraderTest},
    sniffer::{Sniffer as SnifferTest, SnifferBuilder, OrderWithId as TestOrderWithId},
    accountant::Accountant as AccountantTest,
};
use open_midas::{
    calculators::{amount_calculator::AmountCalculator, price_calculator::PriceCalculator},
    limit_master::{LimitMaster, MerchantIdManager},
};
use std::sync::Arc;

#[derive(Default)]
pub struct LimitMasterTestContext {
    pub traders: Vec<Arc<TradesLogger>>,
    pub merchants: Vec<Arc<dyn Merchant>>,
}

impl LimitMasterTestContext {
    pub fn append(
        &mut self,
        trades: Vec<Trade>,
        sniffer: Arc<dyn Sniffer>,
        accountant: Arc<dyn Accountant>,
    ) {
        let trader = Arc::new(TradesLogger::with_orders(
            TraderTest::default(),
            trades));
        self.traders.push(trader.clone());
        self.merchants.push(Arc::new(MerchantTest::custom(
            accountant,
            sniffer,
            trader)));
    }

    pub fn merchants(&self) -> Vec<&dyn Merchant> {
        self.merchants.iter().map(AsRef::as_ref).collect()
    }
}

fn default_buy_trading_pair() -> TradingPair {
    TradingPair {
        coins: Coins::TonUsdt,
        side: Side::Buy,
        target: Target::Limit,
    }
}

fn create_limit_trade(trading_pair: TradingPair, id: u32) -> Trade {
    Trade::Limit(OrderWithId {
        id: id.to_string(),
        trading_pair,
        price: 1.00f64,
        amount: 100f64,
    })
}

#[test]
fn default_limit_master() {
    let trading_pair = default_buy_trading_pair();
    let trade_buy = create_limit_trade(trading_pair, 1337);
    let mut test_context = LimitMasterTestContext::default();
    test_context.append(
        vec![trade_buy.clone()],
        Arc::new(SnifferTest::default()),
        Arc::new(AccountantTest::default())
    );
    test_context.append(
        vec![trade_buy.clone()],
        Arc::new(SnifferTest::default()),
        Arc::new(AccountantTest::default())
    );
    let merchants = test_context.merchants();
    let merchants_manager = MerchantIdManager::new(&merchants);
    let price_calculator = PriceCalculator {
        profit: 0.3f64,
    };
    let amount_calculator = AmountCalculator {
        min_amount_threshold: 1f64,
        fee: 0.01f64
    };
    let mut limit_master = LimitMaster::new(
        Coins::TonUsdt,
        merchants_manager,
        price_calculator,
        amount_calculator,
    );
    let trades = limit_master.check_current_orders();
    let trades = tokio_test::block_on(trades);
    assert!(trades.is_ok());
    let result = limit_master.update_orders();
    let result = tokio_test::block_on(result);
    assert!(result.is_ok());

    for trader in test_context.traders.iter() {
        assert_eq!(trader.create_order_log.lock().unwrap().len(), 1);
    }
}

#[test]
fn update_and_check() {
    let trading_pair = default_buy_trading_pair();
    let sniffer_builder = SnifferBuilder::new()
        .my_orders(vec![TestOrderWithId {
            id: 1337.to_string(),
            price: 1.00f64,
            amount: 100f64,
        }]);
    let amount = 100f64;
    let mut test_context = LimitMasterTestContext::default(); 
    test_context.append(
        vec![
            create_limit_trade(trading_pair.clone(), 1337),
        ],
        Arc::new(sniffer_builder.clone().build(amount)),
        Arc::new(AccountantTest::default()));
    let sniffer_builder = sniffer_builder.my_orders(vec![TestOrderWithId {
            id: 1338.to_string(),
            price: 1.00f64,
            amount: 100f64,
        }]);
    test_context.append(
        vec![
            create_limit_trade(trading_pair.clone(), 1338),
        ],
        Arc::new(sniffer_builder.clone().build(amount)),
        Arc::new(AccountantTest::default()));
    let sniffer_builder = sniffer_builder.my_orders(vec![TestOrderWithId {
            id: 1339.to_string(),
            price: 1.00f64,
            amount: 100f64,
        }]);
    test_context.append(
        vec![
            create_limit_trade(trading_pair.clone(), 1339),
        ],
        Arc::new(sniffer_builder.clone().build(amount)),
        Arc::new(AccountantTest::default()));
    let merchants = test_context.merchants();
    let merchants_manager = MerchantIdManager::new(&merchants);
    let price_calculator = PriceCalculator {
        profit: 0.3f64,
    };
    let amount_calculator = AmountCalculator {
        min_amount_threshold: 1f64,
        fee: 0.01f64
    };
    let mut limit_master = LimitMaster::new(
        Coins::TonUsdt,
        merchants_manager,
        price_calculator,
        amount_calculator,
    );
    let trades = limit_master.check_current_orders();
    let trades = tokio_test::block_on(trades);
    assert!(trades.is_ok());
    assert_eq!(trades.unwrap().len(), 0);

    let result = limit_master.update_orders();
    let result = tokio_test::block_on(result);
    assert!(result.is_ok(), "{:#?}", result);

    for trader in test_context.traders.iter() {
        assert_eq!(trader.create_order_log.lock().unwrap().len(), 3);
    }

    let trades = limit_master.check_current_orders();
    let trades = tokio_test::block_on(trades);
    assert!(trades.is_ok());
    assert_eq!(trades.clone().unwrap().len(), 6, "{:#?}", trades);
}
