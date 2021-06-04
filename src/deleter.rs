use agnostic::{
    merchant::Merchant,
    trading_pair::{TradingPair, Coins, Side, Target}
}; 

#[derive(Debug, Default, Clone)]
pub struct Deleter {
}

impl Deleter {
    pub async fn delete(
        &self,
        merchants: &[&dyn Merchant],
        trading_pair: TradingPair,
    ) -> Result<(), String> {
        for merchant in merchants {
            let sniffer = merchant.sniffer();
            let my_orders = sniffer.get_my_orders(trading_pair.clone()).await?;
            let trader = merchant.trader();
            for order in my_orders.iter() {
                trader.delete_order(&order.id).await?;
            }
        }
        Ok(())
    }

    pub async fn delete_all(
        &self,
        merchants: &[&dyn Merchant],
        coins: Coins,
    ) -> Result<(), String> {
        let trading_pair = TradingPair {
            coins,
            side: Side::Sell,
            target: Target::Limit,
        };
        self.delete(merchants, trading_pair.clone()).await?;
        self.delete(merchants, trading_pair.reversed_side()).await
    }
}
