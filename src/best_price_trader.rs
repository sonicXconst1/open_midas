pub struct BestPriceTrader {
    pub pair: agnostic::trading_pair::TradingPair,
    pub part_of_base_coin_balance: f64,
}

impl BestPriceTrader {
    pub async fn iterate(
        &self,
        merchant: std::sync::Arc<dyn agnostic::merchant::Merchant>
    ) -> Result<(), String> {
        let accountant = merchant.accountant();
        let base_currency = accountant.ask(self.pair.coins.base_coin()).await?;
        let sniffer = merchant.sniffer();
        let mut best_order = sniffer.the_best_order(self.pair.clone()).await?;
        best_order.amount = base_currency.amount * self.part_of_base_coin_balance;
        let trader = merchant.trader();
        let created_trade = trader.create_trade_from_order(best_order).await?;
        log::debug!("Created trade: {:#?}", created_trade);
        Ok(())
    }
}
