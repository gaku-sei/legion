//! Tools for controlling behavior in an ECS application.
//!
//! Systems define how an ECS based application behaves. They have to be
//! registered to a [`SystemStage`](crate::schedule::SystemStage) to be able to
//! run. A system is usually written as a normal function that will be
//! automatically converted into a system.
//!
//! System functions can have parameters, through which one can query and mutate
//! Legion ECS state. Only types that implement [`SystemParam`] can be used,
//! automatically fetching data from the [`World`](crate::world::World).
//!
//! System functions often look like this:
//!
//! ```
//! # use lgn_ecs::prelude::*;
//! #
//! # #[derive(Component)]
//! # struct Player { alive: bool }
//! # #[derive(Component)]
//! # struct Score(u32);
//! # struct Round(u32);
//! #
//! fn update_score_system(
//!     mut query: Query<(&Player, &mut Score)>,
//!     mut round: ResMut<Round>,
//! ) {
//!     for (player, mut score) in query.iter_mut() {
//!         if player.alive {
//!             score.0 += round.0;
//!         }
//!     }
//!     round.0 += 1;
//! }
//! # update_score_system.system();
//! ```
//!
//! # System ordering
//!
//! While the execution of systems is usually parallel and not deterministic,
//! there are two ways to determine a certain degree of execution order:
//!
//! - **System Stages:** They determine hard execution synchronization
//!   boundaries inside of which systems run in parallel by default.
//! - **Labeling:** First, systems are labeled upon creation by calling
//!   `.label()`. Then, methods such as `.before()` and `.after()` are appended
//!   to systems to determine execution order in respect to other systems.
//!
//! # System parameter list
//! Following is the complete list of accepted types as system parameters:
//!
//! - [`Query`]
//! - [`Res`] and `Option<Res>`
//! - [`ResMut`] and `Option<ResMut>`
//! - [`Commands`]
//! - [`Local`]
//! - [`EventReader`](crate::event::EventReader)
//! - [`EventWriter`](crate::event::EventWriter)
//! - [`NonSend`] and `Option<NonSend>`
//! - [`NonSendMut`] and `Option<NonSendMut>`
//! - [`RemovedComponents`]
//! - [`SystemChangeTick`]
//! - [`Archetypes`](crate::archetype::Archetypes) (Provides Archetype metadata)
//! - [`Bundles`](crate::bundle::Bundles) (Provides Bundles metadata)
//! - [`Components`](crate::component::Components) (Provides Components
//!   metadata)
//! - [`Entities`](crate::entity::Entities) (Provides Entities metadata)
//! - All tuples between 1 to 16 elements where each element implements
//!   [`SystemParam`]
//! - [`()` (unit primitive type)](https://doc.rust-lang.org/stable/std/primitive.unit.html)

mod commands;
mod exclusive_system;
mod function_system;
mod query;
#[allow(clippy::module_inception)]
mod system;
mod system_chaining;
mod system_param;

pub use commands::*;
pub use exclusive_system::*;
pub use function_system::*;
pub use query::*;
pub use system::*;
pub use system_chaining::*;
pub use system_param::*;

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use crate::{
        self as lgn_ecs,
        archetype::Archetypes,
        bundle::Bundles,
        component::{Component, Components},
        entity::{Entities, Entity},
        query::{Added, Changed, Or, QueryState, With, Without},
        schedule::{Schedule, Stage, SystemStage},
        system::{
            ConfigurableSystem, IntoExclusiveSystem, IntoSystem, Local, NonSend, NonSendMut, Query,
            QuerySet, RemovedComponents, Res, ResMut, System, SystemState,
        },
        world::{FromWorld, World},
    };

    #[derive(Component, Debug, Eq, PartialEq, Default)]
    struct A;
    #[derive(Component)]
    struct B;
    #[derive(Component)]
    struct C;
    #[derive(Component)]
    struct D;
    #[derive(Component)]
    struct E;
    #[derive(Component)]
    struct F;

    #[derive(Component)]
    struct W<T>(T);

    #[test]
    fn simple_system() {
        fn sys(query: Query<'_, '_, &A>) {
            for a in query.iter() {
                println!("{:?}", a);
            }
        }

        let mut system = sys.system();
        let mut world = World::new();
        world.spawn().insert(A);

        system.initialize(&mut world);
        for archetype in world.archetypes.iter() {
            system.new_archetype(archetype);
        }
        system.run((), &mut world);
    }

    fn run_system<Param, S: IntoSystem<(), (), Param>>(world: &mut World, system: S) {
        let mut schedule = Schedule::default();
        let mut update = SystemStage::parallel();
        update.add_system(system);
        schedule.add_stage("update", update);
        schedule.run(world);
    }

    #[test]
    fn query_system_gets() {
        fn query_system(
            mut ran: ResMut<'_, bool>,
            entity_query: Query<'_, '_, Entity, With<A>>,
            b_query: Query<'_, '_, &B>,
            a_c_query: Query<'_, '_, (&A, &C)>,
            d_query: Query<'_, '_, &D>,
        ) {
            let entities = entity_query.iter().collect::<Vec<Entity>>();
            assert!(
                b_query.get_component::<B>(entities[0]).is_err(),
                "entity 0 should not have B"
            );
            assert!(
                b_query.get_component::<B>(entities[1]).is_ok(),
                "entity 1 should have B"
            );
            assert!(
                b_query.get_component::<A>(entities[1]).is_err(),
                "entity 1 should have A, but b_query shouldn't have access to it"
            );
            assert!(
                b_query.get_component::<D>(entities[3]).is_err(),
                "entity 3 should have D, but it shouldn't be accessible from b_query"
            );
            assert!(
                b_query.get_component::<C>(entities[2]).is_err(),
                "entity 2 has C, but it shouldn't be accessible from b_query"
            );
            assert!(
                a_c_query.get_component::<C>(entities[2]).is_ok(),
                "entity 2 has C, and it should be accessible from a_c_query"
            );
            assert!(
                a_c_query.get_component::<D>(entities[3]).is_err(),
                "entity 3 should have D, but it shouldn't be accessible from b_query"
            );
            assert!(
                d_query.get_component::<D>(entities[3]).is_ok(),
                "entity 3 should have D"
            );

            *ran = true;
        }

        let mut world = World::default();
        world.insert_resource(false);
        world.spawn().insert_bundle((A,));
        world.spawn().insert_bundle((A, B));
        world.spawn().insert_bundle((A, C));
        world.spawn().insert_bundle((A, D));

        run_system(&mut world, query_system);

        assert!(*world.get_resource::<bool>().unwrap(), "system ran");
    }

    #[test]
    fn or_query_set_system() {
        // Regression test for issue #762
        #[allow(clippy::type_complexity)]
        fn query_system(
            mut ran: ResMut<'_, bool>,
            mut set: QuerySet<
                '_,
                '_,
                (
                    QueryState<(), Or<(Changed<A>, Changed<B>)>>,
                    QueryState<(), Or<(Added<A>, Added<B>)>>,
                ),
            >,
        ) {
            let changed = set.q0().iter().count();
            let added = set.q1().iter().count();

            assert_eq!(changed, 1);
            assert_eq!(added, 1);

            *ran = true;
        }

        let mut world = World::default();
        world.insert_resource(false);
        world.spawn().insert_bundle((A, B));

        run_system(&mut world, query_system);

        assert!(*world.get_resource::<bool>().unwrap(), "system ran");
    }

    #[test]
    fn changed_resource_system() {
        struct Added(usize);
        struct Changed(usize);
        fn incr_e_on_flip(
            value: Res<'_, bool>,
            mut changed: ResMut<'_, Changed>,
            mut added: ResMut<'_, Added>,
        ) {
            if value.is_added() {
                added.0 += 1;
            }

            if value.is_changed() {
                changed.0 += 1;
            }
        }

        let mut world = World::default();
        world.insert_resource(false);
        world.insert_resource(Added(0));
        world.insert_resource(Changed(0));

        let mut schedule = Schedule::default();
        let mut update = SystemStage::parallel();
        update.add_system(incr_e_on_flip);
        schedule.add_stage("update", update);
        schedule.add_stage(
            "clear_trackers",
            SystemStage::single(World::clear_trackers.exclusive_system()),
        );

        schedule.run(&mut world);
        assert_eq!(world.get_resource::<Added>().unwrap().0, 1);
        assert_eq!(world.get_resource::<Changed>().unwrap().0, 1);

        schedule.run(&mut world);
        assert_eq!(world.get_resource::<Added>().unwrap().0, 1);
        assert_eq!(world.get_resource::<Changed>().unwrap().0, 1);

        *world.get_resource_mut::<bool>().unwrap() = true;
        schedule.run(&mut world);
        assert_eq!(world.get_resource::<Added>().unwrap().0, 1);
        assert_eq!(world.get_resource::<Changed>().unwrap().0, 2);
    }

    #[test]
    #[should_panic]
    fn conflicting_query_mut_system() {
        fn sys(_q1: Query<'_, '_, &mut A>, _q2: Query<'_, '_, &mut A>) {}

        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn disjoint_query_mut_system() {
        fn sys(_q1: Query<'_, '_, &mut A, With<B>>, _q2: Query<'_, '_, &mut A, Without<B>>) {}

        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn disjoint_query_mut_read_component_system() {
        fn sys(_q1: Query<'_, '_, (&mut A, &B)>, _q2: Query<'_, '_, &mut A, Without<B>>) {}

        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic]
    fn conflicting_query_immut_system() {
        fn sys(_q1: Query<'_, '_, &A>, _q2: Query<'_, '_, &mut A>) {}

        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn query_set_system() {
        fn sys(mut _set: QuerySet<'_, '_, (QueryState<&mut A>, QueryState<&A>)>) {}
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic]
    fn conflicting_query_with_query_set_system() {
        fn sys(
            _query: Query<'_, '_, &mut A>,
            _set: QuerySet<'_, '_, (QueryState<&mut A>, QueryState<&B>)>,
        ) {
        }

        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic]
    fn conflicting_query_sets_system() {
        fn sys(
            _set_1: QuerySet<'_, '_, (QueryState<&mut A>,)>,
            _set_2: QuerySet<'_, '_, (QueryState<&mut A>, QueryState<&B>)>,
        ) {
        }

        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[derive(Default)]
    struct BufferRes {
        _buffer: Vec<u8>,
    }

    fn test_for_conflicting_resources<Param, S: IntoSystem<(), (), Param>>(sys: S) {
        let mut world = World::default();
        world.insert_resource(BufferRes::default());
        world.insert_resource(A);
        world.insert_resource(B);
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic]
    fn conflicting_system_resources() {
        fn sys(_: ResMut<'_, BufferRes>, _: Res<'_, BufferRes>) {}
        test_for_conflicting_resources(sys);
    }

    #[test]
    #[should_panic]
    fn conflicting_system_resources_reverse_order() {
        fn sys(_: Res<'_, BufferRes>, _: ResMut<'_, BufferRes>) {}
        test_for_conflicting_resources(sys);
    }

    #[test]
    #[should_panic]
    fn conflicting_system_resources_multiple_mutable() {
        fn sys(_: ResMut<'_, BufferRes>, _: ResMut<'_, BufferRes>) {}
        test_for_conflicting_resources(sys);
    }

    #[test]
    fn nonconflicting_system_resources() {
        fn sys(
            _: Local<'_, BufferRes>,
            _: ResMut<'_, BufferRes>,
            _: Local<'_, A>,
            _: ResMut<'_, A>,
        ) {
        }
        test_for_conflicting_resources(sys);
    }

    #[test]
    fn local_system() {
        let mut world = World::default();
        world.insert_resource(1u32);
        world.insert_resource(false);
        struct Foo {
            value: u32,
        }

        impl FromWorld for Foo {
            fn from_world(world: &mut World) -> Self {
                Self {
                    value: *world.get_resource::<u32>().unwrap() + 1,
                }
            }
        }

        fn sys(local: Local<'_, Foo>, mut modified: ResMut<'_, bool>) {
            assert_eq!(local.value, 2);
            *modified = true;
        }

        run_system(&mut world, sys);

        // ensure the system actually ran
        assert!(*world.get_resource::<bool>().unwrap());
    }

    #[test]
    fn non_send_option_system() {
        let mut world = World::default();

        world.insert_resource(false);
        struct NotSend1(std::rc::Rc<i32>);
        struct NotSend2(std::rc::Rc<i32>);
        world.insert_non_send(NotSend1(std::rc::Rc::new(0)));

        fn sys(
            op: Option<NonSend<'_, NotSend1>>,
            mut _op2: Option<NonSendMut<'_, NotSend2>>,
            mut run: ResMut<'_, bool>,
        ) {
            op.expect("NonSend should exist");
            *run = true;
        }

        run_system(&mut world, sys);
        // ensure the system actually ran
        assert!(*world.get_resource::<bool>().unwrap());
    }

    #[test]
    fn non_send_system() {
        let mut world = World::default();

        world.insert_resource(false);
        struct NotSend1(std::rc::Rc<i32>);
        struct NotSend2(std::rc::Rc<i32>);

        world.insert_non_send(NotSend1(std::rc::Rc::new(1)));
        world.insert_non_send(NotSend2(std::rc::Rc::new(2)));

        fn sys(
            _op: NonSend<'_, NotSend1>,
            mut _op2: NonSendMut<'_, NotSend2>,
            mut run: ResMut<'_, bool>,
        ) {
            *run = true;
        }

        run_system(&mut world, sys);
        assert!(*world.get_resource::<bool>().unwrap());
    }

    #[test]
    fn removal_tracking() {
        let mut world = World::new();

        let entity_to_despawn = world.spawn().insert(W(1)).id();
        let entity_to_remove_w_from = world.spawn().insert(W(2)).id();
        let spurious_entity = world.spawn().id();

        // Track which entities we want to operate on
        struct Despawned(Entity);
        world.insert_resource(Despawned(entity_to_despawn));
        struct Removed(Entity);
        world.insert_resource(Removed(entity_to_remove_w_from));

        // Verify that all the systems actually ran
        #[derive(Default)]
        struct NSystems(usize);
        world.insert_resource(NSystems::default());

        // First, check that removal detection is triggered if and only if we despawn an entity with the correct component
        world.entity_mut(entity_to_despawn).despawn();
        world.entity_mut(spurious_entity).despawn();

        fn validate_despawn(
            removed_i32: RemovedComponents<'_, W<i32>>,
            despawned: Res<'_, Despawned>,
            mut n_systems: ResMut<'_, NSystems>,
        ) {
            assert_eq!(
                removed_i32.iter().collect::<Vec<_>>(),
                &[despawned.0],
                "despawning causes the correct entity to show up in the 'RemovedComponent' system parameter."
            );

            n_systems.0 += 1;
        }

        run_system(&mut world, validate_despawn);

        // Reset the trackers to clear the buffer of removed components
        // Ordinarily, this is done in a system added by MinimalPlugins
        world.clear_trackers();

        // Then, try removing a component
        world.spawn().insert(W(3)).id();
        world.spawn().insert(W(4)).id();
        world.entity_mut(entity_to_remove_w_from).remove::<W<i32>>();

        fn validate_remove(
            removed_i32: RemovedComponents<'_, W<i32>>,
            removed: Res<'_, Removed>,
            mut n_systems: ResMut<'_, NSystems>,
        ) {
            assert_eq!(
                removed_i32.iter().collect::<Vec<_>>(),
                &[removed.0],
                "removing a component causes the correct entity to show up in the 'RemovedComponent' system parameter."
            );

            n_systems.0 += 1;
        }

        run_system(&mut world, validate_remove);

        // Verify that both systems actually ran
        assert_eq!(world.get_resource::<NSystems>().unwrap().0, 2);
    }

    #[test]
    fn configure_system_local() {
        let mut world = World::default();
        world.insert_resource(false);
        fn sys(local: Local<'_, usize>, mut modified: ResMut<'_, bool>) {
            assert_eq!(*local, 42);
            *modified = true;
        }

        run_system(&mut world, sys.config(|config| config.0 = Some(42)));

        // ensure the system actually ran
        assert!(*world.get_resource::<bool>().unwrap());
    }

    #[test]
    fn world_collections_system() {
        let mut world = World::default();
        world.insert_resource(false);
        world.spawn().insert_bundle((W(42), W(true)));
        fn sys(
            archetypes: &Archetypes,
            components: &Components,
            entities: &Entities,
            bundles: &Bundles,
            query: Query<'_, '_, Entity, With<W<i32>>>,
            mut modified: ResMut<'_, bool>,
        ) {
            assert_eq!(query.iter().count(), 1, "entity exists");
            for entity in query.iter() {
                let location = entities.get(entity).unwrap();
                let archetype = archetypes.get(location.archetype_id).unwrap();
                let archetype_components = archetype.components().collect::<Vec<_>>();
                let bundle_id = bundles
                    .get_id(std::any::TypeId::of::<(W<i32>, W<bool>)>())
                    .expect("Bundle used to spawn entity should exist");
                let bundle_info = bundles.get(bundle_id).unwrap();
                let mut bundle_components = bundle_info.components().to_vec();
                bundle_components.sort();
                for component_id in &bundle_components {
                    assert!(
                        components.get_info(*component_id).is_some(),
                        "every bundle component exists in Components"
                    );
                }
                assert_eq!(
                    bundle_components, archetype_components,
                    "entity's bundle components exactly match entity's archetype components"
                );
            }
            *modified = true;
        }

        run_system(&mut world, sys);

        // ensure the system actually ran
        assert!(*world.get_resource::<bool>().unwrap());
    }

    #[test]
    fn get_system_conflicts() {
        fn sys_x(_: Res<'_, A>, _: Res<'_, B>, _: Query<'_, '_, (&C, &D)>) {}

        fn sys_y(_: Res<'_, A>, _: ResMut<'_, B>, _: Query<'_, '_, (&C, &mut D)>) {}

        let mut world = World::default();
        let mut x = sys_x.system();
        let mut y = sys_y.system();
        x.initialize(&mut world);
        y.initialize(&mut world);

        let conflicts = x.component_access().get_conflicts(y.component_access());
        let b_id = world
            .components()
            .get_resource_id(TypeId::of::<B>())
            .unwrap();
        let d_id = world.components().get_id(TypeId::of::<D>()).unwrap();
        assert_eq!(conflicts, vec![b_id, d_id]);
    }

    #[test]
    fn query_is_empty() {
        fn without_filter(not_empty: Query<'_, '_, &A>, empty: Query<'_, '_, &B>) {
            assert!(!not_empty.is_empty());
            assert!(empty.is_empty());
        }

        fn with_filter(not_empty: Query<'_, '_, &A, With<C>>, empty: Query<'_, '_, &A, With<D>>) {
            assert!(!not_empty.is_empty());
            assert!(empty.is_empty());
        }

        let mut world = World::default();
        world.spawn().insert(A).insert(C);

        let mut without_filter = without_filter.system();
        without_filter.initialize(&mut world);
        without_filter.run((), &mut world);

        let mut with_filter = with_filter.system();
        with_filter.initialize(&mut world);
        with_filter.run((), &mut world);
    }

    #[test]
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn can_have_16_parameters() {
        fn sys_x(
            _: Res<'_, A>,
            _: Res<'_, B>,
            _: Res<'_, C>,
            _: Res<'_, D>,
            _: Res<'_, E>,
            _: Res<'_, F>,
            _: Query<'_, '_, &A>,
            _: Query<'_, '_, &B>,
            _: Query<'_, '_, &C>,
            _: Query<'_, '_, &D>,
            _: Query<'_, '_, &E>,
            _: Query<'_, '_, &F>,
            _: Query<'_, '_, (&A, &B)>,
            _: Query<'_, '_, (&C, &D)>,
            _: Query<'_, '_, (&E, &F)>,
        ) {
        }
        fn sys_y(
            _: (
                Res<'_, A>,
                Res<'_, B>,
                Res<'_, C>,
                Res<'_, D>,
                Res<'_, E>,
                Res<'_, F>,
                Query<'_, '_, &A>,
                Query<'_, '_, &B>,
                Query<'_, '_, &C>,
                Query<'_, '_, &D>,
                Query<'_, '_, &E>,
                Query<'_, '_, &F>,
                Query<'_, '_, (&A, &B)>,
                Query<'_, '_, (&C, &D)>,
                Query<'_, '_, (&E, &F)>,
            ),
        ) {
        }
        let mut world = World::default();
        let mut x = sys_x.system();
        let mut y = sys_y.system();
        x.initialize(&mut world);
        y.initialize(&mut world);
    }

    #[test]
    #[allow(clippy::type_complexity)]
    fn read_system_state() {
        #[derive(Eq, PartialEq, Debug)]
        struct A(usize);

        #[derive(Component, Eq, PartialEq, Debug)]
        struct B(usize);

        let mut world = World::default();
        world.insert_resource(A(42));
        world.spawn().insert(B(7));

        let mut system_state: SystemState<(
            Res<'_, A>,
            Query<'_, '_, &B>,
            QuerySet<'_, '_, (QueryState<&C>, QueryState<&D>)>,
        )> = SystemState::new(&mut world);
        let (a, query, _) = system_state.get(&world);
        assert_eq!(*a, A(42), "returned resource matches initial value");
        assert_eq!(
            *query.single(),
            B(7),
            "returned component matches initial value"
        );
    }

    #[test]
    fn write_system_state() {
        #[derive(Eq, PartialEq, Debug)]
        struct A(usize);

        #[derive(Component, Eq, PartialEq, Debug)]
        struct B(usize);

        let mut world = World::default();
        world.insert_resource(A(42));
        world.spawn().insert(B(7));

        let mut system_state: SystemState<(ResMut<'_, A>, Query<'_, '_, &mut B>)> =
            SystemState::new(&mut world);

        // The following line shouldn't compile because the parameters used are not
        // ReadOnlySystemParam let (a, query) = system_state.get(&world);

        let (a, mut query) = system_state.get_mut(&mut world);
        assert_eq!(*a, A(42), "returned resource matches initial value");
        assert_eq!(
            *query.single_mut(),
            B(7),
            "returned component matches initial value"
        );
    }

    #[test]
    fn system_state_change_detection() {
        #[derive(Component, Eq, PartialEq, Debug)]
        struct A(usize);

        let mut world = World::default();
        let entity = world.spawn().insert(A(1)).id();

        let mut system_state: SystemState<Query<'_, '_, &A, Changed<A>>> =
            SystemState::new(&mut world);
        {
            let query = system_state.get(&world);
            assert_eq!(*query.single(), A(1));
        }

        {
            let query = system_state.get(&world);
            assert!(query.get_single().is_err());
        }

        world.entity_mut(entity).get_mut::<A>().unwrap().0 = 2;
        {
            let query = system_state.get(&world);
            assert_eq!(*query.single(), A(2));
        }
    }

    #[test]
    #[should_panic]
    fn system_state_invalid_world() {
        let mut world = World::default();
        let mut system_state = SystemState::<Query<'_, '_, &A>>::new(&mut world);
        let mismatched_world = World::default();
        system_state.get(&mismatched_world);
    }

    #[test]
    fn system_state_archetype_update() {
        #[derive(Component, Eq, PartialEq, Debug)]
        struct A(usize);

        #[derive(Component, Eq, PartialEq, Debug)]
        struct B(usize);

        let mut world = World::default();
        world.spawn().insert(A(1));

        let mut system_state = SystemState::<Query<'_, '_, &A>>::new(&mut world);
        {
            let query = system_state.get(&world);
            assert_eq!(
                query.iter().collect::<Vec<_>>(),
                vec![&A(1)],
                "exactly one component returned"
            );
        }

        world.spawn().insert_bundle((A(2), B(2)));
        {
            let query = system_state.get(&world);
            assert_eq!(
                query.iter().collect::<Vec<_>>(),
                vec![&A(1), &A(2)],
                "components from both archetypes returned"
            );
        }
    }

    /// this test exists to show that read-only world-only queries can return
    /// data that lives as long as 'world
    #[test]
    #[allow(unused)]
    fn long_life_test() {
        struct Holder<'w> {
            value: &'w A,
        }

        struct State {
            state: SystemState<Res<'static, A>>,
            state_q: SystemState<Query<'static, 'static, &'static A>>,
        }

        impl State {
            fn hold_res<'w>(&mut self, world: &'w World) -> Holder<'w> {
                let a = self.state.get(world);
                Holder {
                    value: a.into_inner(),
                }
            }
            fn hold_component<'w>(&mut self, world: &'w World, entity: Entity) -> Holder<'w> {
                let q = self.state_q.get(world);
                let a = q.get(entity).unwrap();
                Holder { value: a }
            }
            fn hold_components<'w>(&mut self, world: &'w World) -> Vec<Holder<'w>> {
                let mut components = Vec::new();
                let q = self.state_q.get(world);
                for a in q.iter() {
                    components.push(Holder { value: a });
                }
                components
            }
        }
    }

    #[test]
    fn immutable_mut_test() {
        #[derive(Component, Eq, PartialEq, Debug, Clone, Copy)]
        struct A(usize);

        let mut world = World::default();
        world.spawn().insert(A(1));
        world.spawn().insert(A(2));

        let mut system_state = SystemState::<Query<'_, '_, &mut A>>::new(&mut world);
        {
            let mut query = system_state.get_mut(&mut world);
            assert_eq!(
                query.iter_mut().map(|m| *m).collect::<Vec<A>>(),
                vec![A(1), A(2)],
                "both components returned by iter_mut of &mut"
            );
            assert_eq!(
                query.iter().collect::<Vec<&A>>(),
                vec![&A(1), &A(2)],
                "both components returned by iter of &mut"
            );
        }
    }
}