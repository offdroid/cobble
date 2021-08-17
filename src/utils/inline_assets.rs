// Adaptation of https://github.com/emosenkis/bevy_prototype_inline_assets/blob/5b5f70f4a5507777443b10cda3972e3c7695af56/src/lib.rs
// Licensed under MIT, see LICENSES/bevy_prototype_inline_assets/ for more information
use bevy::asset::FileAssetIo;

use bevy::{
    asset::{AssetIo, AssetIoError, AssetServerSettings, BoxedFuture},
    prelude::*,
    tasks::IoTaskPool,
};
use futures::future::TryFutureExt;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct InlineAssets {
    assets: HashMap<&'static Path, &'static [u8]>,
}

#[allow(dead_code)]
impl InlineAssets {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    pub fn load_all(
        &self,
        asset_server: Res<AssetServer>,
    ) -> HashMap<&'static Path, HandleUntyped> {
        self.assets
            .keys()
            .map(|&p| (p, asset_server.load_untyped(p)))
            .collect()
    }

    pub fn add(&mut self, path: &'static Path, data: &'static [u8]) -> &mut Self {
        self.assets.insert(path, data);
        self
    }
    fn io<T: AssetIo>(&self, base: T) -> InlineAssetIo {
        InlineAssetIo {
            assets: self.assets.clone(),
            base: Box::new(base),
        }
    }
}

#[macro_export]
macro_rules! inline_assets {
    [$($paths:literal),+,] => {
        inline_assets![$($paths),+]
    };
    [$($paths:literal),+] => {{
        let mut inline_assets = $crate::utils::inline_assets::InlineAssets::new();
        $( inline_assets.add(std::path::Path::new($paths), &include_bytes!(concat!("../", $paths))[..]) );+;
        inline_assets
    }};
}

struct InlineAssetIo {
    assets: HashMap<&'static Path, &'static [u8]>,
    base: Box<dyn AssetIo>,
}

impl AssetIo for InlineAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        let future = self.base.load_path(path);
        if let Some(&bytes) = self.assets.get(path) {
            Box::pin(future.or_else(move |_| futures::future::ok(bytes.to_owned())))
        } else {
            future
        }
    }
    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        self.base.read_directory(path)
    }
    fn is_directory(&self, path: &Path) -> bool {
        self.base.is_directory(path)
    }
    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError> {
        self.base.watch_path_for_changes(path)
    }
    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        self.base.watch_for_changes()
    }
}

#[derive(Default)]
pub struct InlineAssetsPlugin;

impl Plugin for InlineAssetsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let task_pool: IoTaskPool = (*app
            .world()
            .get_resource::<IoTaskPool>()
            .expect("IoTaskPool resource not found"))
        .clone();

        let base_asset_io = {
            let settings = app
                .world_mut()
                .get_resource_or_insert_with(AssetServerSettings::default);

            FileAssetIo::new(&settings.asset_folder)
        };

        let asset_io = app
            .world()
            .get_resource::<InlineAssets>()
            .expect("InlineAssets resource not found")
            .io(base_asset_io);

        app.insert_resource(AssetServer::new(asset_io, task_pool.0));
    }
}
