use agnostic::trading_pair::TradingPair;
use agnostic::merchant::Merchant;
use agnostic::trade::Trade;
use std::sync::Arc;

pub struct BestPriceMarketTrader {
    pub pair: TradingPair,
    pub amount: f64,
}

impl BestPriceMarketTrader {
    pub async fn iterate(&self, merchant: Arc<dyn Merchant>) -> Result<Trade, String> {
        let sniffer = merchant.sniffer();
        let mut best_order = sniffer.all_the_best_orders(self.pair.clone(), 1)
            .await?
            .remove(0);
        best_order.amount = self.amount;
        let trader = merchant.trader();
        trader.create_order(best_order).await
    }
}
