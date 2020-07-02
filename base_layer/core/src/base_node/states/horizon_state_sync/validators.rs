//  Copyright 2020, The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

mod headers;
pub use headers::HorizonHeadersValidator;

mod chain_balance;
pub use chain_balance::{ChainBalanceValidator, HeaderIter};

mod mmr_roots;
pub use mmr_roots::MmrRootsValidator;

use crate::{
    blocks::BlockHeader,
    chain_storage::{BlockchainBackend, BlockchainDatabase},
    consensus::ConsensusManager,
    transactions::types::CryptoFactories,
    validation::{StatelessValidation, StatelessValidationExt, StatelessValidator},
};
use std::{fmt, sync::Arc};

#[derive(Clone)]
pub struct HorizonSyncValidators {
    pub header: Arc<StatelessValidator<BlockHeader>>,
    pub final_state: Arc<StatelessValidator<u64>>,
}

impl HorizonSyncValidators {
    pub fn new<THeader, TFinal>(header: THeader, final_state: TFinal) -> Self
    where
        THeader: StatelessValidation<BlockHeader> + 'static,
        TFinal: StatelessValidation<u64> + 'static,
    {
        Self {
            header: Arc::new(Box::new(header)),
            final_state: Arc::new(Box::new(final_state)),
        }
    }

    pub fn full_consensus<B: BlockchainBackend + 'static>(
        db: BlockchainDatabase<B>,
        rules: ConsensusManager,
        factories: CryptoFactories,
    ) -> Self
    {
        Self::new(
            HorizonHeadersValidator::new(db.clone(), rules.clone()),
            ChainBalanceValidator::new(db.clone(), rules.clone(), factories).chain(MmrRootsValidator::new(db, rules)),
        )
    }
}

impl fmt::Debug for HorizonSyncValidators {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HorizonHeaderValidators")
            .field("header", &"...")
            .field("final_state", &"...")
            .finish()
    }
}
