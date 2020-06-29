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
    chain_storage::{BlockchainBackend, BlockchainDatabase, ChainStorageError},
    consensus::ConsensusManager,
    transactions::{
        tari_amount::MicroTari,
        types::{BlindingFactor, Commitment, CryptoFactories, PrivateKey},
    },
    validation::{StatelessValidation, ValidationError},
};
use log::*;
use std::cmp;
use tari_crypto::commitment::HomomorphicCommitmentFactory;

const LOG_TARGET: &str = "c::bn::states::horizon_state_sync::chain_balance";

/// Validate that the chain balances at a given height.
pub struct ChainBalanceValidator<B> {
    rules: ConsensusManager,
    db: BlockchainDatabase<B>,
    factories: CryptoFactories,
}

impl<B: BlockchainBackend> ChainBalanceValidator<B> {
    pub fn new(db: BlockchainDatabase<B>, rules: ConsensusManager, factories: CryptoFactories) -> Self {
        Self { db, rules, factories }
    }
}

impl<B: BlockchainBackend> StatelessValidation<u64> for ChainBalanceValidator<B> {
    fn validate(&self, horizon_height: &u64) -> Result<(), ValidationError> {
        let total_offset = self.fetch_total_offset_commitment(*horizon_height)?;
        let emission_h = self.get_emission_commitment_at(*horizon_height);
        let kernel_excess = self.fetch_aggregate_kernel_excess()?;
        let genesis_input_commit = self.get_aggregate_genesis_commitment();
        let output = self.fetch_aggregate_utxo_commitment()?;

        // Validate: ∑UTXO_i ?= Emission + ∑GENESIS_COMMIT_i + ∑KERNEL_EXCESS_i + ∑OFFSET_i
        let agg_excess = &kernel_excess + &genesis_input_commit;
        let input = &(&emission_h + &agg_excess) + &total_offset;

        if output != input {
            return Err(ValidationError::custom_error(format!(
                "Final state validation failed: The UTXO set did not balance with the expected emission at height {}",
                horizon_height
            )));
        }

        Ok(())
    }
}

impl<B: BlockchainBackend> ChainBalanceValidator<B> {
    fn fetch_total_offset_commitment(&self, height: u64) -> Result<Commitment, ValidationError> {
        let header_iter = HeaderIter::new(&self.db, height, 50);
        let mut total_offset = BlindingFactor::default();
        let mut count = 0u64;
        for header in header_iter {
            let header = header.map_err(ValidationError::custom_error)?;
            count += 1;
            total_offset = total_offset + header.total_kernel_offset;
        }
        trace!(target: LOG_TARGET, "Fetched {} headers", count);
        let offset_commitment = self.factories.commitment.commit(&total_offset, &0u64.into());
        Ok(offset_commitment)
    }

    fn fetch_aggregate_utxo_commitment(&self) -> Result<Commitment, ValidationError> {
        let utxos = self.db.fetch_all_utxos().map_err(ValidationError::custom_error)?;
        trace!(target: LOG_TARGET, "Fetched {} UTXOs", utxos.len());
        Ok(utxos.into_iter().map(|u| u.commitment).sum())
    }

    fn get_emission_commitment_at(&self, height: u64) -> Commitment {
        let total_supply = self.rules.emission_schedule().supply_at_block(height) -
            self.rules.consensus_constants().get_genesis_coinbase_value_offset();
        trace!(
            target: LOG_TARGET,
            "Expected emission at height {} is {}",
            height,
            total_supply
        );
        self.commit_value(total_supply)
    }

    fn get_aggregate_genesis_commitment(&self) -> Commitment {
        // Get the sum of unspent genesis block UTXOs (excl coinbase)
        self.rules
            .get_genesis_block()
            .body
            .outputs()
            .into_iter()
            .filter(|u| !u.is_coinbase())
            .map(|u| &u.commitment)
            .sum()
    }

    fn fetch_aggregate_kernel_excess(&self) -> Result<Commitment, ValidationError> {
        let kernels = self.db.fetch_all_kernels().map_err(ValidationError::custom_error)?;
        trace!(target: LOG_TARGET, "Fetched {} kernels", kernels.len());
        Ok(kernels.into_iter().map(|k| k.excess).sum())
    }

    #[inline]
    fn commit_value(&self, v: MicroTari) -> Commitment {
        self.factories.commitment.commit_value(&PrivateKey::default(), v.into())
    }
}

// TODO: This is probably generally useful and can be generalized for any DB "item" that we want to load in chunks
/// Iterator that emits BlockHeaders until a given height. This iterator loads headers in chunks of size `chunk_size`
/// for a low memory footprint. The chunk buffer is allocated once and reused.
pub struct HeaderIter<'a, B> {
    chunk: Vec<BlockHeader>,
    chunk_size: usize,
    cursor: usize,
    is_error: bool,
    height: u64,
    db: &'a BlockchainDatabase<B>,
}

impl<'a, B> HeaderIter<'a, B> {
    pub fn new(db: &'a BlockchainDatabase<B>, height: u64, chunk_size: usize) -> Self {
        Self {
            db,
            chunk_size,
            cursor: 0,
            is_error: false,
            height,
            chunk: Vec::with_capacity(chunk_size),
        }
    }

    fn next_chunk(&self) -> Vec<u64> {
        let upper_bound = cmp::min(self.cursor + self.chunk_size, self.height as usize);
        (self.cursor..=upper_bound).map(|n| n as u64).collect()
    }
}

impl<B: BlockchainBackend> Iterator for HeaderIter<'_, B> {
    type Item = Result<BlockHeader, ChainStorageError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_error {
            return None;
        }

        if self.chunk.is_empty() {
            let block_nums = self.next_chunk();
            // We're done: No more block headers to fetch
            if block_nums.is_empty() {
                return None;
            }

            match self.db.fetch_headers(block_nums) {
                Ok(headers) => {
                    self.cursor += headers.len();
                    self.chunk.extend(headers);
                },
                Err(err) => {
                    // On the next call, the iterator will end
                    self.is_error = true;
                    return Some(Err(err));
                },
            }
        }

        Some(Ok(self.chunk.remove(0)))
    }
}
