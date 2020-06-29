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

use crate::{
    blocks::BlockHeader,
    chain_storage::{BlockchainBackend, BlockchainDatabase, MmrTree},
    consensus::ConsensusManager,
    validation::{StatelessValidation, ValidationError},
};
use log::*;

const LOG_TARGET: &str = "c::bn::states::horizon_state_sync::mmr_roots";

pub struct MmrRootsValidator<B> {
    rules: ConsensusManager,
    db: BlockchainDatabase<B>,
}

impl<B: BlockchainBackend> MmrRootsValidator<B> {
    pub fn new(db: BlockchainDatabase<B>, rules: ConsensusManager) -> Self {
        Self { db, rules }
    }

    fn check_utxo_mr(&self, tip_header: &BlockHeader) -> Result<(), ValidationError> {
        let node_count = self
            .db
            .fetch_mmr_node_count(MmrTree::Utxo, tip_header.height)
            .map_err(ValidationError::custom_error)?;

        let (additions, deletions) = self
            .db
            .fetch_mmr_nodes(MmrTree::Utxo, 0, node_count, None)
            .map_err(ValidationError::custom_error)?
            .into_iter()
            .partition::<Vec<_>, _>(|(_, is_stxo)| !*is_stxo);
        let additions = additions.into_iter().map(|(hash, _)| hash).collect();
        let deletions = deletions.into_iter().map(|(hash, _)| hash).collect();
        let output_mr = self
            .db
            .calculate_mmr_root(MmrTree::Utxo, additions, deletions)
            .map_err(ValidationError::custom_error)?;
        if tip_header.output_mr != output_mr {
            return Err(ValidationError::InvalidOutputMr);
        }
        Ok(())
    }

    fn check_kernel_mr(&self, tip_header: &BlockHeader) -> Result<(), ValidationError> {
        let node_count = self
            .db
            .fetch_mmr_node_count(MmrTree::Kernel, tip_header.height)
            .map_err(ValidationError::custom_error)?;

        let hashes = self
            .db
            .fetch_mmr_nodes(MmrTree::Kernel, 0, node_count, None)
            .map_err(ValidationError::custom_error)?
            .into_iter()
            .map(|(hash, _)| hash)
            .collect();
        let kernel_mr = self
            .db
            .calculate_mmr_root(MmrTree::Kernel, hashes, vec![])
            .map_err(ValidationError::custom_error)?;
        if tip_header.kernel_mr != kernel_mr {
            return Err(ValidationError::InvalidKernelMr);
        }
        Ok(())
    }
}

impl<B: BlockchainBackend> StatelessValidation<u64> for MmrRootsValidator<B> {
    fn validate(&self, _horizon_height: &u64) -> Result<(), ValidationError> {
        // TODO: Check MRs
        // let tip_header = self
        //     .db
        //     .fetch_header(*horizon_height)
        //     .map_err(ValidationError::custom_error)?;
        // debug!(
        //     target: LOG_TARGET,
        //     "Validating MMR roots for horizon state at height {}", tip_header.height
        // );

        // self.check_kernel_mr(&tip_header)?;
        // self.check_utxo_mr()?;

        Ok(())
    }
}
