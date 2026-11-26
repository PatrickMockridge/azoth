# HAZOP basis — AZ-FEED-08 Rev B (frozen for review)

## Design intent
Offshore gas-condensate platform, 60/10/2 three-stage separation, manned. At year ten the facility,
not the reservoir, limits production: 137.1 MMscfd sales gas, 2,936 bbl/d condensate, 2,782 bbl/d
produced water (48.7 % cut). The recommended debottlenecking scope is the **HIPPS + produced-water
package**: compressor re-rate (+18 %), HP separator internals (K 0.11 → 0.14 m/s), HIPPS SIL-3 with a
35 % blocked-outlet relief credit, and a produced-water upgrade from 3,000 to 6,000 bbl/d with a
coalescer. Sales gas rises to 161.8 MMscfd and condensate to about 3,466 bbl/d. Four-week shutdown.

## Battery limits
Wellhead choke outlet (95 bara, 363.15 K) → HP inlet SDV-101 → V-101 → MP/LP let-down and separation →
three compression trains to 90 bara → sales gas metering. Condensate from the stock tank to export.
Produced water from V-101 to the water package and overboard/reinjection (outside battery limits).
Relief from V-101 via PSV-101 to the existing flare. Export gas and condensate lines leave the platform.

## Key equipment and data
| Tag | Service | Data |
|---|---|---|
| V-101 | HP 3-phase separator | Ø1.96 × 7.26 m T/T, 66 barg, 100 °C, SA-516-70N + 3 mm CA, 316L internals, K 0.14 m/s |
| V-102 | MP separator | 10 bara, existing, 3 mm CA |
| V-103 | LP separator | 2 bara, existing, 3 mm CA |
| V-104 | Stock tank / condensate flash | 1.013 bara, 288.15 K |
| K-101 | HP gas compressor | 60→90 bara, 1 stage |
| K-102 A/B/C | MP gas compressor | 10→90 bara, 3 stages, intercooled to 313.15 K |
| K-103 A–E | LP gas compressor | 2→90 bara, 5 stages, intercooled to 313.15 K |
| SDV-101 | HIPPS final element | 10 in, 316L, SIL-3, 1oo2 PT, PFD 1e-4, proof test 12 months |
| PSV-101 | HP relief | P orifice, 6.38 in² fitted, 3.60 in² required with HIPPS, set 66 barg |
| V-104/V-105 | Hydrocyclone + coalescer | 6,000 bbl/d, 25 barg, 316L/Super Duplex |
| P-102 | Treated-water pump | 39.7 m³/h at 60 m, 15 kW |
| Flare | Existing flare and header | 46.34 kg/s design; 69.9 % at HIPPS relief case; radiation 64.0 m to 4.73 kW/m² |

## Relief and safety case
Governing case: HP separator blocked outlet, 42.63 kg/s at 72.6 bara. With the HIPPS 35 % credit:
32.70 kg/s, flare 69.9 %, PSV 3.60 in², radiation 64.0 m. Fire case 0.54 kg/s. HIPPS: NOG-070 minimum
SIL 3, achieved SIL 3, 1oo2, HFT 1, DC 0.6; pipeline maximum incidental 72.6 bara, two barriers
required and provided; S-001 acceptable with relief leakage assessed and routed safe. The 35 % credit
is a study assumption, not a library result.

## Materials
Carbon steel with 3.0 mm CA and continuous CO2 inhibition for wet hydrocarbon service (CO2 pp 0.732 bar,
81 °C). 316L for produced water, HIPPS wetted parts and instrument connections. No HAZOP-relevant H2S.
Export gas line 12 in X65, 12.7 mm WT, 3LPP; condensate 6 in X65; water 8 in 316L.

## Operating envelope
Sales gas 137.1–161.8 MMscfd; condensate 2,936–3,466 bbl/d; water 2,782 bbl/d rising with cut to 57 %;
HP 60 bara; export 90 bara; turndown to 60 % of design; ambient -5 to 35 °C; shutdown 4 weeks.

## HAZOP nodes
1. HP inlet, SDV-101 and HIPPS (wellhead choke to V-101)
2. V-101 HP three-phase separator (including water draw-off)
3. MP let-down: VLV-102 and V-102
4. LP let-down and stock tank: VLV-103, V-103, E-104, V-104
5. Compression: K-101, K-102 A/B/C, K-103 A–E, inter/aftercoolers, sales gas mixer
6. Produced-water package: V-104 hydrocyclone, V-105 coalescer, P-102
7. Relief, PSV-101 and flare
8. Export gas and condensate lines (battery limit to reception)

## Guidewords
NO / NOT, MORE (flow, pressure, temperature, level), LESS, REVERSE, AS WELL AS (contaminants,
water, other phase), PART OF, OTHER THAN (startup, shutdown, maintenance, utility failure,
instrument failure). Use only where meaningful.

## Requested output
A HAZOP worksheet: node, guideword, deviation, cause, consequence, existing safeguard, severity (S1-S4),
likelihood (L1-L4), risk (low/medium/high), action, action owner, status. Flag any finding that
invalidates the design basis or the 35 % HIPPS credit. Do not invent library results; refer to the
report for numbers.
