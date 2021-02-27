pub struct BestPriceMarketTrader {
    pair: agnostic::trading_pair::TradingPair,
    amount: f64,
}

impl BestPriceMarketTrader {
    pub async fn iterate(
        &self,
        merchant: std::sync::Arc<dyn agnostic::merchant::Merchant>,
    ) -> Result<(), String> {
        let sniffer = merchant.sniffer();
        let mut best_order = sniffer.the_best_order(self.pair.clone()).await?;
        best_order.amount = self.amount;
        let trader = merchant.trader();
        match self.pair.target {
            agnostic::trading_pair::Target::Market => {
                trader.create_trade_from_order(best_order).await
            },
            agnostic::trading_pair::Target::Limit => {
                trader.create_order(best_order).await
            },
        }
    }
}
