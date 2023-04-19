use std::mem::MaybeUninit;

use bytemuck::{AnyBitPattern, NoUninit};
use game_common::components::components::{Component, RecordReference};
use game_common::entity::EntityId;
use game_common::events::Event;
use game_common::world::entity::EntityBody;
use game_common::world::world::{WorldState, WorldViewMut};
use game_wasm::log::Level;
use game_wasm::raw;
use game_wasm::raw::world::{Entity, Item};
use wasmtime::{
    AsContextMut, Caller, Engine, Func, FuncType, Instance, Linker, Module, Store, TypedFunc, Val,
};

use crate::events::{Events, OnAction, OnCollision};

macro_rules! register_fns {
    ($linker:expr, $($id:ident),*$(,)?) => {
        $(
            $linker.func_wrap("host", stringify!($id), $id);
        )*
    };
}

pub struct ScriptInstance<'world> {
    store: Store<State<'world>>,
    inner: Instance,
    events: Events,
}

impl<'world> ScriptInstance<'world> {
    pub fn new(
        engine: &Engine,
        module: &Module,
        events: Events,
        world: WorldViewMut<'world>,
    ) -> Self {
        let mut store = Store::new(engine, State { world });

        //  let imports = vec![Func::new(
        //      &store,
        //      FuncType::new(ValType::I32, ValType::I32),
        //      |caller, params, results| {
        //          dbg!(params);
        //          Ok(())
        //      },
        //  )];

        //  let imports = vec![Func::wrap(&mut store, |i: i32| -> i32 {
        //      dbg!(i);
        //      i * 2
        //  })];

        let mut linker = Linker::<State>::new(&engine);

        // linker.func_wrap(
        //     "host",
        //     "hello",
        //     |caller: Caller<'_, ()>, param: u32| -> u32 {
        //         println!("got {} hello world ", param);
        //         0
        //     },
        // );

        register_fns! {
            linker,
            log,
            world_entity_get,
            world_entity_spawn,
            world_entity_despawn,
        };

        let instance = linker.instantiate(&mut store, &module).unwrap();

        Self {
            store,
            inner: instance,
            events,
        }
    }

    // pub fn run(&mut self) {
    //     let main: TypedFunc<u32, u32> = self
    //         .inner
    //         .get_typed_func(&mut self.store, "script_main")
    //         .unwrap();

    //     dbg!("calling");
    //     let out = main.call(&mut self.store, 23).unwrap();
    //     dbg!(out);
    // }

    pub fn run(&mut self, event: &Event) {
        tracing::info!("exec {:?}", event);

        match event {
            Event::Action(event) => self.on_action(event.entity, event.invoker),
            Event::Collision { entity, other } => self.on_collision(*entity, *other),
        }
    }

    pub fn on_action(&mut self, entity: EntityId, invoker: EntityId) {
        let func: OnAction = self
            .inner
            .get_typed_func(&mut self.store, "on_action")
            .unwrap();

        func.call(&mut self.store, (entity.into_raw(), invoker.into_raw()))
            .unwrap();
    }

    pub fn on_collision(&mut self, entity: EntityId, other: EntityId) {
        let func: OnCollision = self
            .inner
            .get_typed_func(&mut self.store, "on_collision")
            .unwrap();

        func.call(&mut self.store, (entity.into_raw(), other.into_raw()))
            .unwrap();
    }
}

fn log(mut caller: Caller<'_, State<'_>>, level: u32, ptr: u32, len: u32) -> wasmtime::Result<()> {
    let mut memory = caller.get_export("memory").unwrap().into_memory().unwrap();

    let bytes = memory.data(&caller);

    let mut buf = &bytes[ptr as usize..ptr as usize + len as usize];

    let content = std::str::from_utf8(buf).unwrap();

    match Level::from_raw(level) {
        Level::ERROR => {
            tracing::error!("{}", content)
        }
        Level::WARN => {
            tracing::warn!("{}", content);
        }
        Level::INFO => {
            tracing::info!("{}", content)
        }
        Level::DEBUG => {
            tracing::info!("{}", content)
        }
        Level::TRACE => {
            tracing::info!("{}", content)
        }
        _ => unreachable!(),
    }

    Ok(())
}

fn world_entity_spawn(mut caller: Caller<'_, State<'_>>, ptr: u32) -> u32 {
    0
}

fn world_entity_get(mut caller: Caller<'_, State<'_>>, id: u64, out: u32) -> u32 {
    let Some(entity) = caller.data_mut().world.get(EntityId::from_raw(id)) else {
        return 1;
    };

    let ent = Entity {
        id,
        translation: entity.transform.translation.to_array(),
        rotation: entity.transform.rotation.to_array(),
        scale: entity.transform.scale.to_array(),
        body: match &entity.body {
            EntityBody::Item(item) => raw::world::EntityBody::Item(Item {
                id: game_wasm::raw::record::RecordReference {
                    module: item.id.0.module.into_bytes(),
                    record: item.id.0.record,
                },
            }),
            EntityBody::Actor(_) => raw::world::EntityBody::Actor,
            EntityBody::Object(_) => raw::world::EntityBody::Object,
            EntityBody::Terrain(_) => raw::world::EntityBody::Terrain,
        },
    };

    let bytes = unsafe { std::mem::transmute::<Entity, [u8; std::mem::size_of::<Entity>()]>(ent) };

    let memory = caller.get_export("memory").unwrap().into_memory().unwrap();

    let buf = memory.data_mut(caller.as_context_mut());

    let buf = &mut buf[out as usize..out as usize + std::mem::size_of::<Entity>()];
    buf.copy_from_slice(&bytes);

    0
}

fn world_entity_despawn(mut caller: Caller<'_, State<'_>>, id: u64) -> u32 {
    let id = EntityId::from_raw(id);

    caller.data_mut().world.despawn(id);
    0
}

fn world_entity_component_get(
    mut caller: Caller<'_, State<'_>>,
    entity_id: u64,
    component_id: u32,
    out: u32,
) -> wasmtime::Result<u32> {
    let entity_id = EntityId::from_raw(entity_id);
    let component_id: RecordReference = read_memory(&mut caller, component_id)?;

    let entity = caller.data_mut().world.get(entity_id).unwrap();

    let Some(comp) = entity.components.get(component_id) else {
        return Ok(1);
    };

    let comp: game_wasm::world::Component = match comp {
        Component::I32(x) => game_wasm::world::Component::I32(*x),
        Component::I64(x) => game_wasm::world::Component::I64(*x),
    };

    let b: [u8; std::mem::size_of::<game_wasm::world::Component>()] =
        unsafe { std::mem::transmute(comp) };

    let mut memory = caller.get_export("memory").unwrap().into_memory().unwrap();
    let bytes = memory.data_mut(&mut caller);

    let out = &mut bytes[out as usize..out as usize + b.len()];
    out.copy_from_slice(&b);

    // write_memory(&mut caller, out, comp)?;
    Ok(0)
}

// fn world_entity_component_insert(
//     mut caller: &mut Caller<'_, State<'_>>,
//     entity_id: u64,
//     component_id: u32,
//     ptr: u32,
// ) -> wasmtime::Result<u32> {
//     let entity_id = EntityId::from_raw(entity_id);
//     let component_id: RecordReference = read_memory(&mut caller, component_id)?;

//     let entity = caller.data_mut().world.get(entity_id).unwrap();

//     let component = read_memory(&mut caller, ptr)?;

//     entity.components.insert(
//         component_id,
//         match component {
//             game_wasm::world::Component::I32(x) => Component::I32(x),
//             game_wasm::world::Component::I64(x) => Component::I64(x),
//         },
//     );

//     Ok(0)
// }

/// Reads type `T` from the guest memory starting at `T`.
fn read_memory<T: AnyBitPattern>(
    caller: &mut Caller<'_, State<'_>>,
    ptr: u32,
) -> wasmtime::Result<T> {
    let mut memory = caller.get_export("memory").unwrap().into_memory().unwrap();
    let bytes = memory.data(&caller);

    let size = std::mem::size_of::<T>();
    let bytes = &bytes[ptr as usize..ptr as usize + size as usize];

    Ok(bytemuck::pod_read_unaligned(bytes))
}

// Why does T: Uninit imply T: Pod?
fn write_memory<T: NoUninit>(
    mut caller: &mut Caller<'_, State<'_>>,
    ptr: u32,
    value: &T,
) -> wasmtime::Result<()> {
    let mut memory = caller.get_export("memory").unwrap().into_memory().unwrap();
    let bytes = memory.data_mut(&mut caller);

    let size = std::mem::size_of::<T>();
    let out = &mut bytes[ptr as usize..ptr as usize + size as usize];

    out.copy_from_slice(bytemuck::bytes_of(value));

    Ok(())
}

pub struct State<'world> {
    world: WorldViewMut<'world>,
}
