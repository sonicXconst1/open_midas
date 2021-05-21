use open_midas::{
    limit_master::{
        LimitMaster,
        MerchantIdManager,
    },
    calculators::{
        amount_calculator::AmountCalculator,
        price_calculator::PriceCalculator,
    }
};
use agnostic::{
    trading_pair::{
        Coins,
    },
    merchant::Merchant
};
use agnostic_test::{
    merchant::Merchant as MerchantTest
};

#[tokio::test]
async fn default_limit_master() {
    let merchants: [&dyn Merchant; 4] = [
        &MerchantTest::default(),
        &MerchantTest::default(),
        &MerchantTest::default(),
        &MerchantTest::default(),
    ];
    let merchants_manager = MerchantIdManager::new(&merchants);
    let price_calculator = PriceCalculator {
        profit: 0.3f64
    };
    let amount_calculator = AmountCalculator {
        min_amount_threshold: 10f64,
        fee: 0.01,
    };
    let mut limit_master = LimitMaster::new(
        Coins::TonUsdt,
        merchants_manager,
        price_calculator,
        amount_calculator
    );
    let trades = limit_master.check_current_orders().await;
    assert!(trades.is_ok());
    let result = limit_master.update_orders().await;
    assert!(result.is_ok());
}
