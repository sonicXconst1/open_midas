use agnostic::order::Order;

#[derive(Clone, Copy, Debug)]
pub struct LowAmountFilter {
    pub low_amount: f64,
}

impl LowAmountFilter {
    pub fn filter(&self, order: Vec<Order>) -> Vec<Order> {
        order.into_iter()
            .filter(|order| order.amount <= self.low_amount)
            .collect()
    }
}
