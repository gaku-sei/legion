use std::sync::Arc;

use strum_macros::{Display, EnumString};

mod atlas;
mod collision;
pub mod compiler;
mod database;
mod expression;
mod meta;
mod navmesh;
mod package;
mod runtime_dependency;
mod rust_yard;
mod texture;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentAddr(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString, Display)]
pub enum Platform {
    PS5,
    //XSX,
    XB1,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString, Display)]
pub enum Target {
    Client,
    Server,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString, Display)]
pub enum Locale {
    English,
    French,
    Spanish,
    Japenese,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuildParams {
    pub platform: Platform,
    pub target: Target,
    pub locale: Locale,
}

impl BuildParams {
    pub fn new(platform: Platform, target: Target, locale: Locale) -> Arc<Self> {
        Arc::new(Self {
            platform,
            target,
            locale,
        })
    }
}

impl Default for BuildParams {
    fn default() -> Self {
        Self {
            platform: Platform::PS5,
            target: Target::Client,
            locale: Locale::English,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompilerError {
    ParsingError,
}

#[cfg(test)]
pub mod tests {
    use crate::{compiler::Compiler, database::DatabaseImpl};

    pub fn setup() -> DatabaseImpl {
        let mut db = DatabaseImpl::default();

        db.set_read(
            "TextureA.meta".to_string(),
            "Default:TextureA.jpg".to_string(), // In real implementation it would be JSON
        );
        db.set_read("TextureA.jpg".to_string(), "Texture A".to_string());

        db.set_read(
            "TextureB.meta".to_string(),
            "Default:TextureB.png".to_string(), // In real implementation it would be JSON
        );
        db.set_read("TextureB.png".to_string(), "Texture B".to_string());

        db.set_read(
            "TextureC.meta".to_string(),
            "English:TextureCEn.jpg;French:TextureCFr.png".to_string(),
        );
        db.set_read(
            "TextureCEn.jpg".to_string(),
            "Texture in English".to_string(),
        );
        db.set_read(
            "TextureCFr.png".to_string(),
            "Texture en Français".to_string(),
        );

        db.set_read(
            "Atlas.atlas".to_string(),
            "meta(read(TextureA.meta));meta(read(TextureB.meta));meta(read(TextureC.meta))"
                .to_string(),
        );

        db.set_read(
            "MyWorld.entity".to_string(),
            r#"atlas(read(Atlas.atlas));exec(read(Car.coll));exec(read(Tree.coll))"#.to_string(),
        );

        db.set_read("Car.coll".to_string(), "aabb(5,5,5,10,10,10)".to_string());
        db.set_read(
            "Tree.coll".to_string(),
            "aabb(30,30,30,50,60,70)".to_string(),
        );

        db
    }

    #[test]
    fn compile_all() {
        let db = setup();

        db.package_see_ps5();
    }
}
