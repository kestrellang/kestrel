use std::fmt::Debug;

use crate::language::Language;

pub trait Behavior<L: Language>: Debug + Send + Sync + downcast_rs::DowncastSync {
    fn kind(&self) -> L::BehaviorKind;
}

// Add downcast support to the Behavior trait
// Note: We use basic (non-sync) downcasting because downcast_rs doesn't support
// sync downcasting with generic type parameters. Use .as_ref() to downcast Arc.
downcast_rs::impl_downcast!(sync Behavior<L> where L: Language);
