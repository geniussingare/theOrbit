# Gestures

TheOrbit watches the right mouse button and nothing else. It stays out of the way until you hold the button down.

## The gestures

| What you do | What happens |
| --- | --- |
| Right-click and release quickly | The normal context menu opens. TheOrbit replays the click to the app. |
| Hold the right button (350 ms by default) | The ring opens, centred on the pointer. |
| Move onto an item, then release | That item runs. |
| Release in the centre | Nothing runs. The ring closes. |
| Right-drag before the hold delay ends | Passed through untouched, so games, CAD tools and gesture apps keep working. |

!!! note "The context menu appears on release"
    TheOrbit holds the right-button press until it knows what you meant. A normal context menu therefore opens when you let go, not when you press down.

## How selection works

The ring is split into equal segments, one per item.

- The first item is centred at **12 o'clock**. The rest follow **clockwise**.
- The centre of the ring is a **cancel zone** (42 px by default). Releasing inside it runs nothing.
- Selection follows direction, not distance. If you move beyond the outer edge of the ring, the segment in that direction still counts, like a classic pie menu.
- While the ring is open, the mouse moves the highlight and does not reach the app underneath.

With four items the segments sit at top, right, bottom and left. With eight, they sit at each compass point and in between.

## Tune the feel

| Setting | Default | Make it... |
| --- | --- | --- |
| `holdMs` | 350 | Longer if the ring opens by accident. Shorter if it feels slow. |
| `dragThreshold` | 8 | Larger to forgive a shaky hand. Smaller to pass drags through sooner. |
| `deadZone` | 42 | Larger for a bigger cancel area. |
| `radius` | 150 | Larger to give each segment more room. |

Hold delay is also a slider in the settings window. All values are described in the [configuration reference](../reference/configuration.md).

## Turn it off for a while

Choose **Orbit Enabled** from the tray icon to switch TheOrbit off and on. When it is off, every mouse event passes straight through.

## Motion

The ring springs open, the highlight glides between items, and the ring swells away when you choose one. If you turn on **Reduce motion** in your operating system, TheOrbit follows that setting. An optional soft tick confirms a choice. Turn it off with `sound` in the config or **Sound on select** in the settings window.
