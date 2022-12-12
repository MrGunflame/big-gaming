
# Server/Client

Attempt to minimize the amount of traffic while resulting in a stable, deterministic game
on all clients. For most gameplay operations the server should be authoritative and validate
inputs as long as possible without sacrificing much performance.

## Header

```
|
| Version (8b) | R |

R: Retransmitted
```

## Events

An actor (NPC/Player) changed position. The body contains the new absolute position.
```
struct ActorMove {
    x: f32,
    y: f32,
    z: f32,
}
```

An actor attacks with the currently equipped weapon.
```
struct ActorAttack {
}
```
