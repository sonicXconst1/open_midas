use agnostic::merchant_manager;
use agnostic::merchant_manager::IdentityGiver;
use agnostic::merchant::Merchant;
use std::sync::Arc;
use std::collections::HashMap;

pub struct MerchantManager<TIdentityGiver: IdentityGiver> {
    giver: TIdentityGiver,
    merchants: HashMap<TIdentityGiver::Identity, Arc<dyn Merchant>>,
}

impl<TIdentityGiver: IdentityGiver> MerchantManager<TIdentityGiver> {
    pub fn new(giver: TIdentityGiver) -> Self {
        MerchantManager {
            giver,
            merchants: Default::default(),
        }
    }
}

impl<TIdentityGiver> merchant_manager::MerchantManager for MerchantManager<TIdentityGiver>
where
    TIdentityGiver: IdentityGiver<Token = Arc<dyn Merchant>>,
    <TIdentityGiver as IdentityGiver>::Identity: std::hash::Hash + Eq + Copy
{
    type Giver = TIdentityGiver;
    type Identity = TIdentityGiver::Identity;

    fn insert(&mut self, merchant: Arc<dyn Merchant>) -> Option<Self::Identity> {
        let id = self.giver.give(merchant.clone());
        self.merchants.insert(id, merchant).map_or(None, |_| Some(id))
    }

    fn remove(&mut self, identity: Self::Identity) -> Option<Arc<dyn Merchant>> {
        self.merchants.remove(&identity)
    }

    fn get(&self, identity: Self::Identity) -> Option<Arc<dyn Merchant>> {
        self.merchants.get(&identity).map(|merchant| merchant.clone())
    }
}
