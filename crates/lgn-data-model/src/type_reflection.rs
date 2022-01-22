/// Trait that implement reflection functions
pub trait TypeReflection {
    /// Return the `TypeDefinition` of the instance
    fn get_type(&self) -> TypeDefinition;

    /// Return the `TypeDefinition` for a Type
    fn get_type_def() -> TypeDefinition
    where
        Self: Sized;

    /// Return the `TypeDefinition` for a Option<Type>
    fn get_option_def() -> TypeDefinition
    where
        Self: Sized,
    {
        TypeDefinition::None
    }
    /// Return the `ArrayDefinition` for a Vec<Type>
    fn get_array_def() -> TypeDefinition
    where
        Self: Sized,
    {
        TypeDefinition::None
    }
}

/// Type Definition
#[derive(Clone, Copy)]
pub enum TypeDefinition {
    /// Invalid Type
    None,
    /// Primitive Type
    Primitive(&'static crate::PrimitiveDescriptor),
    /// Struct Type
    Struct(&'static crate::StructDescriptor),
    /// Array Type
    Array(&'static crate::ArrayDescriptor),
    /// Option Type
    Option(&'static crate::OptionDescriptor),
    /// Box<dyn XXX> Type
    BoxDyn(&'static crate::BoxDynDescriptor),
}

impl TypeDefinition {
    /// Return the name of the type
    pub fn get_type_name(&self) -> &str {
        match *self {
            TypeDefinition::Array(array_descriptor) => {
                array_descriptor.base_descriptor.type_name.as_str()
            }
            TypeDefinition::Struct(struct_descriptor) => {
                struct_descriptor.base_descriptor.type_name.as_str()
            }
            TypeDefinition::Primitive(primitive_descriptor) => {
                primitive_descriptor.base_descriptor.type_name.as_str()
            }
            TypeDefinition::Option(option_descriptor) => {
                option_descriptor.base_descriptor.type_name.as_str()
            }
            TypeDefinition::BoxDyn(box_dyn_descriptor) => box_dyn_descriptor.type_name.as_str(),
            TypeDefinition::None => "None",
        }
    }
}