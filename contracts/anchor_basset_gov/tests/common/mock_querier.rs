use anchor_basset_token::state::{MinterData, TokenInfo};
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_slice, to_binary, Api, Coin, Empty, Extern, FullDelegation, HumanAddr, Querier,
    QuerierResult, QueryRequest, SystemError, Uint128, Validator, WasmQuery,
};
use cosmwasm_storage::to_length_prefixed;
use gov_courier::PoolInfo;
use std::collections::HashMap;

pub const MOCK_CONTRACT_ADDR: &str = "cosmos2contract";

pub fn mock_dependencies(
    canonical_length: usize,
    contract_balance: &[Coin],
) -> Extern<MockStorage, MockApi, WasmMockQuerier> {
    let contract_addr = HumanAddr::from(MOCK_CONTRACT_ADDR);
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new(
        MockQuerier::new(&[(&contract_addr, contract_balance)]),
        canonical_length,
        MockApi::new(canonical_length),
    );

    Extern {
        storage: MockStorage::default(),
        api: MockApi::new(canonical_length),
        querier: custom_querier,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<Empty>,
    canonical_length: usize,
    token_querier: TokenQuerier,
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Raw { contract_addr, key }) => {
                let prefix_pool = to_length_prefixed(b"pool_info").to_vec();
                let prefix_token_inf = to_length_prefixed(b"token_info").to_vec();
                let api: MockApi = MockApi::new(self.canonical_length);
                if key.as_slice().to_vec() == prefix_pool {
                    let pool = PoolInfo {
                        exchange_rate: Default::default(),
                        total_bond_amount: Default::default(),
                        last_index_modification: 0,
                        reward_account: api.canonical_address(&HumanAddr::from("reward")).unwrap(),
                        is_reward_exist: true,
                        is_token_exist: true,
                        token_account: api.canonical_address(&HumanAddr::from("token")).unwrap(),
                    };
                    Ok(to_binary(&to_binary(&pool).unwrap()))
                } else if key.as_slice().to_vec() == prefix_token_inf {
                    let balances: &HashMap<HumanAddr, Uint128> =
                        match self.token_querier.balances.get(contract_addr) {
                            Some(balances) => balances,
                            None => {
                                return Err(SystemError::InvalidRequest {
                                    error: format!(
                                        "No balance info exists for the contract {}",
                                        contract_addr
                                    ),
                                    request: key.as_slice().into(),
                                })
                            }
                        };
                    let mut total_supply = Uint128::zero();

                    for balance in balances {
                        total_supply += *balance.1;
                    }
                    let api: MockApi = MockApi::new(self.canonical_length);
                    let token_inf: TokenInfo = TokenInfo {
                        name: "bluna".to_string(),
                        symbol: "BLUNA".to_string(),
                        decimals: 6,
                        total_supply,
                        mint: Some(MinterData {
                            minter: api
                                .canonical_address(&HumanAddr::from("governance"))
                                .unwrap(),
                            cap: None,
                        }),
                        owner: api
                            .canonical_address(&HumanAddr::from("governance"))
                            .unwrap(),
                    };
                    Ok(to_binary(&to_binary(&token_inf).unwrap()))
                } else {
                    unimplemented!()
                }
            }
            _ => self.base.handle_query(request),
        }
    }
    pub fn update_staking(
        &mut self,
        denom: &str,
        validators: &[Validator],
        delegations: &[FullDelegation],
    ) {
        self.base.update_staking(denom, validators, delegations);
    }
}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    balances: HashMap<HumanAddr, HashMap<HumanAddr, Uint128>>,
}

impl TokenQuerier {
    pub fn new(balances: &[(&HumanAddr, &[(&HumanAddr, &Uint128)])]) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
        }
    }
}

pub(crate) fn balances_to_map(
    balances: &[(&HumanAddr, &[(&HumanAddr, &Uint128)])],
) -> HashMap<HumanAddr, HashMap<HumanAddr, Uint128>> {
    let mut balances_map: HashMap<HumanAddr, HashMap<HumanAddr, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<HumanAddr, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert(HumanAddr::from(addr), **balance);
        }

        balances_map.insert(HumanAddr::from(contract_addr), contract_balances_map);
    }
    balances_map
}

impl WasmMockQuerier {
    pub fn new<A: Api>(base: MockQuerier<Empty>, canonical_length: usize, _api: A) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
            canonical_length,
        }
    }

    // configure the mint whitelist mock querier
    pub fn with_token_balances(&mut self, balances: &[(&HumanAddr, &[(&HumanAddr, &Uint128)])]) {
        self.token_querier = TokenQuerier::new(balances);
    }
}
