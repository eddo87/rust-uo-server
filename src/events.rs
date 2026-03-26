use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Represents all game events that can flow through the event bus.
#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    PlayerConnected { player_id: u32 },
    PlayerDisconnected { player_id: u32 },
    PlayerMoved { player_id: u32, x: i32, y: i32, z: i32 },
    PlayerSpoke { player_id: u32, message: String },
    CombatHit { attacker_id: u32, target_id: u32, damage: u32 },
    CombatMiss { attacker_id: u32, target_id: u32 },
    MobileDied { mobile_id: u32 },
    MobileResurrected { mobile_id: u32 },
    ItemPickedUp { item_id: u32, mobile_id: u32 },
    ItemDropped { item_id: u32, x: i32, y: i32, z: i32 },
    ItemEquipped { item_id: u32, mobile_id: u32, slot: u8 },
    SkillUsed { mobile_id: u32, skill_id: u16 },
    SkillGained { mobile_id: u32, skill_id: u16, new_value: f32 },
    SpellCast { caster_id: u32, spell_id: u16, target_id: Option<u32> },
    ContainerOpened { container_id: u32, opened_by: u32 },
    ServerShutdown,
    WorldSaved,
    WorldLoaded,
}

impl GameEvent {
    /// Returns the event name used for subscription matching.
    pub fn name(&self) -> &'static str {
        match self {
            GameEvent::PlayerConnected { .. } => "PlayerConnected",
            GameEvent::PlayerDisconnected { .. } => "PlayerDisconnected",
            GameEvent::PlayerMoved { .. } => "PlayerMoved",
            GameEvent::PlayerSpoke { .. } => "PlayerSpoke",
            GameEvent::CombatHit { .. } => "CombatHit",
            GameEvent::CombatMiss { .. } => "CombatMiss",
            GameEvent::MobileDied { .. } => "MobileDied",
            GameEvent::MobileResurrected { .. } => "MobileResurrected",
            GameEvent::ItemPickedUp { .. } => "ItemPickedUp",
            GameEvent::ItemDropped { .. } => "ItemDropped",
            GameEvent::ItemEquipped { .. } => "ItemEquipped",
            GameEvent::SkillUsed { .. } => "SkillUsed",
            GameEvent::SkillGained { .. } => "SkillGained",
            GameEvent::SpellCast { .. } => "SpellCast",
            GameEvent::ContainerOpened { .. } => "ContainerOpened",
            GameEvent::ServerShutdown => "ServerShutdown",
            GameEvent::WorldSaved => "WorldSaved",
            GameEvent::WorldLoaded => "WorldLoaded",
        }
    }
}

type EventHandler = Box<dyn Fn(&GameEvent) + Send + 'static>;

struct EventBusInner {
    subscribers: HashMap<String, Vec<EventHandler>>,
}

/// A thread-safe, clone-able event bus for dispatching game events to subscribers.
#[derive(Clone)]
pub struct EventBus {
    inner: Arc<Mutex<EventBusInner>>,
}

impl EventBus {
    /// Creates a new empty event bus.
    pub fn new() -> Self {
        EventBus {
            inner: Arc::new(Mutex::new(EventBusInner {
                subscribers: HashMap::new(),
            })),
        }
    }

    /// Subscribe a handler to a named event. The handler will be called each
    /// time an event with a matching name is published.
    pub fn subscribe<F>(&self, event_name: &str, handler: F)
    where
        F: Fn(&GameEvent) + Send + 'static,
    {
        let mut inner = self.inner.lock().unwrap();
        inner
            .subscribers
            .entry(event_name.to_string())
            .or_default()
            .push(Box::new(handler));
    }

    /// Publish an event, invoking all handlers that are subscribed to its name.
    pub fn publish(&self, event: &GameEvent) {
        let inner = self.inner.lock().unwrap();
        if let Some(handlers) = inner.subscribers.get(event.name()) {
            for handler in handlers {
                handler(event);
            }
        }
    }

    /// Returns the total number of subscriber handlers registered for a given
    /// event name. Returns 0 if no handlers are registered.
    pub fn subscriber_count(&self, event_name: &str) -> usize {
        let inner = self.inner.lock().unwrap();
        inner
            .subscribers
            .get(event_name)
            .map_or(0, |handlers| handlers.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_game_event_names() {
        assert_eq!(GameEvent::PlayerConnected { player_id: 1 }.name(), "PlayerConnected");
        assert_eq!(GameEvent::PlayerDisconnected { player_id: 1 }.name(), "PlayerDisconnected");
        assert_eq!(GameEvent::PlayerMoved { player_id: 1, x: 0, y: 0, z: 0 }.name(), "PlayerMoved");
        assert_eq!(GameEvent::PlayerSpoke { player_id: 1, message: "hi".into() }.name(), "PlayerSpoke");
        assert_eq!(GameEvent::CombatHit { attacker_id: 1, target_id: 2, damage: 10 }.name(), "CombatHit");
        assert_eq!(GameEvent::CombatMiss { attacker_id: 1, target_id: 2 }.name(), "CombatMiss");
        assert_eq!(GameEvent::MobileDied { mobile_id: 1 }.name(), "MobileDied");
        assert_eq!(GameEvent::MobileResurrected { mobile_id: 1 }.name(), "MobileResurrected");
        assert_eq!(GameEvent::ItemPickedUp { item_id: 1, mobile_id: 2 }.name(), "ItemPickedUp");
        assert_eq!(GameEvent::ItemDropped { item_id: 1, x: 0, y: 0, z: 0 }.name(), "ItemDropped");
        assert_eq!(GameEvent::ItemEquipped { item_id: 1, mobile_id: 2, slot: 0 }.name(), "ItemEquipped");
        assert_eq!(GameEvent::SkillUsed { mobile_id: 1, skill_id: 5 }.name(), "SkillUsed");
        assert_eq!(GameEvent::SkillGained { mobile_id: 1, skill_id: 5, new_value: 50.0 }.name(), "SkillGained");
        assert_eq!(GameEvent::SpellCast { caster_id: 1, spell_id: 3, target_id: Some(2) }.name(), "SpellCast");
        assert_eq!(GameEvent::ContainerOpened { container_id: 1, opened_by: 2 }.name(), "ContainerOpened");
        assert_eq!(GameEvent::ServerShutdown.name(), "ServerShutdown");
        assert_eq!(GameEvent::WorldSaved.name(), "WorldSaved");
        assert_eq!(GameEvent::WorldLoaded.name(), "WorldLoaded");
    }

    #[test]
    fn test_new_event_bus_is_empty() {
        let bus = EventBus::new();
        assert_eq!(bus.subscriber_count("PlayerConnected"), 0);
        assert_eq!(bus.subscriber_count("NonExistent"), 0);
    }

    #[test]
    fn test_subscribe_and_count() {
        let bus = EventBus::new();
        bus.subscribe("PlayerConnected", |_| {});
        assert_eq!(bus.subscriber_count("PlayerConnected"), 1);

        bus.subscribe("PlayerConnected", |_| {});
        assert_eq!(bus.subscriber_count("PlayerConnected"), 2);

        bus.subscribe("CombatHit", |_| {});
        assert_eq!(bus.subscriber_count("CombatHit"), 1);
        assert_eq!(bus.subscriber_count("PlayerConnected"), 2);
    }

    #[test]
    fn test_publish_invokes_handler() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicU32::new(0));

        let counter_clone = counter.clone();
        bus.subscribe("PlayerConnected", move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(&GameEvent::PlayerConnected { player_id: 42 });
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        bus.publish(&GameEvent::PlayerConnected { player_id: 43 });
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_publish_does_not_invoke_unrelated_handlers() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicU32::new(0));

        let counter_clone = counter.clone();
        bus.subscribe("CombatHit", move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(&GameEvent::PlayerConnected { player_id: 1 });
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_publish_multiple_handlers_same_event() {
        let bus = EventBus::new();
        let counter_a = Arc::new(AtomicU32::new(0));
        let counter_b = Arc::new(AtomicU32::new(0));

        let a = counter_a.clone();
        bus.subscribe("MobileDied", move |_| { a.fetch_add(1, Ordering::SeqCst); });

        let b = counter_b.clone();
        bus.subscribe("MobileDied", move |_| { b.fetch_add(10, Ordering::SeqCst); });

        bus.publish(&GameEvent::MobileDied { mobile_id: 1 });
        assert_eq!(counter_a.load(Ordering::SeqCst), 1);
        assert_eq!(counter_b.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_handler_receives_correct_event_data() {
        let bus = EventBus::new();
        let received_damage = Arc::new(AtomicU32::new(0));

        let dmg = received_damage.clone();
        bus.subscribe("CombatHit", move |event| {
            if let GameEvent::CombatHit { damage, .. } = event {
                dmg.store(*damage, Ordering::SeqCst);
            }
        });

        bus.publish(&GameEvent::CombatHit {
            attacker_id: 1,
            target_id: 2,
            damage: 55,
        });
        assert_eq!(received_damage.load(Ordering::SeqCst), 55);
    }

    #[test]
    fn test_clone_shares_state() {
        let bus = EventBus::new();
        let bus2 = bus.clone();
        let counter = Arc::new(AtomicU32::new(0));

        let c = counter.clone();
        bus.subscribe("WorldSaved", move |_| { c.fetch_add(1, Ordering::SeqCst); });

        // Publishing through the clone should invoke the handler registered on the original.
        bus2.publish(&GameEvent::WorldSaved);
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Subscriber count visible from both handles.
        assert_eq!(bus.subscriber_count("WorldSaved"), 1);
        assert_eq!(bus2.subscriber_count("WorldSaved"), 1);
    }

    #[test]
    fn test_thread_safety() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicU32::new(0));

        let c = counter.clone();
        bus.subscribe("ServerShutdown", move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        });

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let bus_clone = bus.clone();
                std::thread::spawn(move || {
                    bus_clone.publish(&GameEvent::ServerShutdown);
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_publish_no_subscribers_does_not_panic() {
        let bus = EventBus::new();
        // Should not panic when no handlers are registered.
        bus.publish(&GameEvent::WorldLoaded);
    }

    #[test]
    fn test_game_event_clone_and_debug() {
        let event = GameEvent::SpellCast {
            caster_id: 5,
            spell_id: 42,
            target_id: Some(10),
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("SpellCast"));
    }
}
