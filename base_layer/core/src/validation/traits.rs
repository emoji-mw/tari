// Copyright 2019. The Tari Project
//
// Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
// following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
// disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
// following disclaimer in the documentation and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
// products derived from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use crate::validation::{chained::ChainedValidator, error::ValidationError};

pub type Validator<T, B> = Box<dyn Validation<T, B>>;
pub type StatelessValidator<T> = Box<dyn StatelessValidation<T>>;

/// The core validation trait. Multiple `Validation` implementors can be chained together in a [ValidatorPipeline] to
/// provide consensus validation for blocks, transactions, or DAN instructions. Implementors only need to implement
/// the methods that are relevant for the pipeline, since the default implementation always passes.
pub trait Validation<T, B>: Send + Sync {
    /// General validation code that can run independent of external state
    fn validate(&self, item: &T, db: &B) -> Result<(), ValidationError>;
}

/// Stateless version of the core validation trait.
pub trait StatelessValidation<T>: Send + Sync {
    /// General validation code that can run independent of external state
    fn validate(&self, item: &T) -> Result<(), ValidationError>;
}

pub trait StatelessValidationExt<T>: StatelessValidation<T> {
    /// Consume this validator and create a chained validator that performs this validation followed by another
    fn chain<V: StatelessValidation<T>>(self, other: V) -> ChainedValidator<Self, V>
    where Self: Sized {
        ChainedValidator::new(self, other)
    }
}
impl<T, U> StatelessValidationExt<T> for U where U: StatelessValidation<T> {}
