use cosmwasm_std::{Coin, StdResult};

pub struct Coins(pub Vec<Coin>);

impl Coins {
    pub fn add(&mut self, coin_to_add: &Coin) -> StdResult<()> {
        match self.0.iter_mut().find(|coin| coin.denom == coin_to_add.denom) {
            Some(coin) => {
                coin.amount = coin.amount.checked_add(coin_to_add.amount)?;
            },
            None => {
                self.0.push(coin_to_add.clone());
            },
        }
        Ok(())
    }

    pub fn add_many(&mut self, coins_to_add: &Coins) -> StdResult<()> {
        for coin_to_add in &coins_to_add.0 {
            self.add(coin_to_add)?;
        }
        Ok(())
    }

    pub fn find(&self, denom: &str) -> Coin {
        self.0
            .iter()
            .cloned()
            .find(|coin| coin.denom == denom)
            .unwrap_or_else(|| Coin::new(0, denom))
    }
}
