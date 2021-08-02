//! A crate with modules supporting data compilation process.
//!
//! * [`compiler_api`] provides an interface for implementing a data compiler.
//! * [`compiler_cmd`] provides utilities for interacting with data compilers.
//! * [`compiled_asset_store`] tools for storing and retrieving compiled assets.

// BEGIN - Legion Labs lints v0.2
// do not change or add/remove here, but one can add exceptions after this section
#![deny(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::if_let_mutex,
    clippy::implicit_clone,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::string_add_assign,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::useless_transmute,
    clippy::use_self,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values,
    future_incompatible,
    nonstandard_style,
    broken_intra_doc_links,
    private_intra_doc_links,
    missing_crate_level_docs,
    rust_2018_idioms
)]
// END - Legion Labs standard lints v0.2
// crate-specific exceptions:
#![allow()]
#![warn(missing_docs)]

use compiled_asset_store::CompiledAssetStoreAddr;
use legion_assets::AssetId;
use serde::{Deserialize, Serialize};

/// [`CompilerHash`] identifies an output generated by the data compiler.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Hash)]
pub struct CompilerHash(pub u64);

/// Description of a compiled asset.
///
/// The contained information can be used to retrieve and validate the asset from a [`CompiledAssetStore`](`compiled_asset_store::CompiledAssetStore`).
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct CompiledAsset {
    /// The id of the asset.
    pub guid: AssetId,
    /// The checksum of the asset.
    pub checksum: i128,
    /// The size of the asset.
    pub size: usize,
}

/// The output of data compilation.
///
/// `Manifest` contains the list of compiled assets.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Manifest {
    /// The description of all compiled assets.
    pub compiled_assets: Vec<CompiledAsset>,
}

/// Build target enumeration.
///
/// `TODO`: This needs to be more extensible.
#[derive(Clone, Copy)]
pub enum Target {
    /// Game client.
    Game,
    /// Server.
    Server,
    /// Backend service.
    Backend,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Target::Game => write!(f, "game"),
            Target::Server => write!(f, "server"),
            Target::Backend => write!(f, "backend"),
        }
    }
}

impl FromStr for Target {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "game" => Ok(Self::Game),
            "server" => Ok(Self::Server),
            "backend" => Ok(Self::Backend),
            _ => Err(()),
        }
    }
}

/// Build platform enumeration.
#[derive(Clone, Copy)]
pub enum Platform {
    /// Windows
    Windows,
    /// Unix
    Unix,
    /// Game Console X
    ConsoleX,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Platform::Windows => write!(f, "windows"),
            Platform::Unix => write!(f, "unix"),
            Platform::ConsoleX => write!(f, "consolex"),
        }
    }
}

impl FromStr for Platform {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "windows" => Ok(Self::Windows),
            "unix" => Ok(Self::Unix),
            "consolex" => Ok(Self::ConsoleX),
            _ => Err(()),
        }
    }
}

/// Defines user's language/region.
pub struct Locale(String);

impl Locale {
    /// Creates a new Locale.
    pub fn new(v: &str) -> Self {
        Self(String::from(v))
    }
}

use core::fmt;
use std::str::FromStr;

pub mod compiled_asset_store;
pub mod compiler_api;
pub mod compiler_cmd;
