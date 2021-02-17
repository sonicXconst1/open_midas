pub struct BestPriceTrader {
    pub pair: agnostic::trading_pair::TradingPair,
    pub amount: f64,
}

impl BestPriceTrader {
    pub async fn iterate(
        &self,
        merchant: std::sync::Arc<dyn agnostic::merchant::Merchant>
    ) -> Result<(), String> {
        let sniffer = merchant.sniffer();
        let mut best_order = sniffer.the_best_order(self.pair.clone()).await?;
        best_order.amount = self.amount;
        let trader = merchant.trader();
        let created_trade = trader.create_trade_from_order(best_order).await?;
        log::info!("Created trade: {:#?}", created_trade);
        Ok(())
    }
}
