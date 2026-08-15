# EmberlineDTL

![EmberlineDTL](./assets/banner.png)

[![CI](https://github.com/SolguardLabs/EmberlineDTL/actions/workflows/ci.yml/badge.svg)](https://github.com/SolguardLabs/EmberlineDTL/actions/workflows/ci.yml)
[![Release Integrity](https://github.com/SolguardLabs/EmberlineDTL/actions/workflows/release-integrity.yml/badge.svg)](https://github.com/SolguardLabs/EmberlineDTL/actions/workflows/release-integrity.yml)
[![Rust 1.96](https://img.shields.io/badge/Rust-1.96-000000.svg)](https://www.rust-lang.org/)
[![Node 24](https://img.shields.io/badge/Node-24-5FA04E.svg)](https://nodejs.org/)

EmberlineDTL es un motor determinista de routing y settlement para redes de pago multirregión. Selecciona rutas por coste, latencia y reputación; reserva presupuesto en un pool por activo; liquida incentivos y penalizaciones; y entrega un informe reconciliable con journal y digest de estado.

El núcleo Rust ejecuta sin acceso de red ni estado externo. Los escenarios publicados ofrecen una interfaz reproducible para el plano de control, mientras el cliente JavaScript aporta límites de proceso, validación de respuesta y ejecución sin shell.

## Arquitectura

```mermaid
flowchart LR
    O["Order intake"] --> R["Route planner"]
    R --> K["Risk envelope"]
    K --> P["Settlement policy"]
    P --> B["Budget reservation"]
    B --> E["Execution receipt"]
    E --> Q["Rebate quote"]
    Q --> L["Ledger postings"]
    L --> C["Reconciliation report"]
```

| Componente   | Responsabilidad                                     |
| ------------ | --------------------------------------------------- |
| `route`      | Planes, legs, costes, compensaciones y recibos.     |
| `risk`       | Admisión por envelope, operador y exposición.       |
| `policy`     | Reserva, scoring, rebate, penalización y release.   |
| `pool`       | Contabilidad de presupuesto disponible y reservado. |
| `settlement` | Orquestación de admisión y ejecución atómica.       |
| `ledger`     | Postings deterministas por cuenta y activo.         |
| `treasury`   | Stress de liquidez, concentración y cartera.        |
| `report`     | Contrato JSON, digests e invariantes.               |
| `sdk`        | Cliente JavaScript endurecido para el binario.      |

## Ciclo de una ruta

```mermaid
stateDiagram-v2
    [*] --> Submitted
    Submitted --> Rejected: envelope denegado
    Submitted --> Admitted: política aprobada
    Admitted --> Reserved: presupuesto bloqueado
    Reserved --> Executed: receipt confirmado
    Executed --> Closed: rebate y release
    Rejected --> [*]
    Closed --> [*]
```

```mermaid
sequenceDiagram
    participant C as Control plane
    participant E as EmberlineDTL
    participant P as Rebate pool
    participant O as Operator
    C->>E: submit RoutePlan
    E->>E: risk + policy
    E->>P: reserve budget
    C->>E: SettlementReceipt
    E->>E: score cost and time
    E->>P: close reservation
    P->>O: operator rebate
    E-->>C: report + digest
```

## Cálculo operativo

Las cantidades son enteros `u128` acotados. Los porcentajes se expresan en basis points. El motor calcula primero el coste bruto de los legs y una reserva con buffer y límite por tier:

```text
grossCost = Σ(quotedCost_i + externalFee_i)
reserveBuffer = policyBufferBps + operatorTierBufferBps
reserveAmount = min(grossCost × (1 + reserveBuffer), operatorReserveLimit)
```

En el cierre, coste y tiempo forman el score combinado junto con tier y trust score:

```text
combinedScore = 45% × costScore
              + 35% × timeScore
              + 10% × tierBoost
              + 10% × trustScore
```

```mermaid
flowchart TD
    GC["Gross cost"] --> RS["Reservation"]
    EC["Effective cost"] --> CS["Cost score"]
    TM["Elapsed time"] --> TS["Time score"]
    TR["Tier + trust"] --> SS["Combined score"]
    CS --> SS
    TS --> SS
    SS --> RQ["Rebate quote"]
    RS --> RQ
    RQ --> CL["Close reservation"]
```

## Modelo de tesorería

`src/treasury.rs` evalúa posiciones por activo sin mutar el motor transaccional:

```text
available = floor((poolAvailable + expectedInflow) × (1 - liquidityHaircut))
required = ceil(poolReserved × (1 + reservationShock))
         + ceil(scheduledRebates × (1 + rebateShock))
         + ceil(accumulatedPenalties × (1 + penaltyAddon))
         + ceil(largestReservation × concentrationAddon)
surplus = max(available - required, 0)
shortfall = max(required - available, 0)
```

El resultado incluye coverage, utilización, concentración, bandas `nominal`, `watch`, `guarded`, `critical` y señales deterministas.

## Inicio rápido

Requisitos: Rust `1.96.0`, Node.js `24.x` y Bash.

```bash
npm ci
cargo run --locked -- --list
cargo run --locked -- scenario normal
cargo run --locked -- validate pool
npm run test:all
npm run ci
```

Cliente JavaScript:

```js
import { EmberlineClient } from "./sdk/client.js";

const client = new EmberlineClient({
  binaryPath: "target/release/emberline_dtl",
  timeoutMs: 5_000,
});

const report = client.runScenario("normal");
console.log(report.state_digest, report.pool);
```

En Windows, use `target/release/emberline_dtl.exe`.

## Documentación

- [Arquitectura](docs/01-arquitectura.md)
- [Routing y admisión](docs/02-routing-y-admision.md)
- [Economía de rebates](docs/03-economia-de-rebates.md)
- [Tesorería y stress](docs/04-tesoreria-y-stress.md)
- [Operación](docs/05-operacion.md)
- [Integración y observabilidad](docs/06-integracion-y-observabilidad.md)
- [Despliegue](docs/07-despliegue.md)
- [Política de seguridad](SECURITY.md)

## Garantías de ingeniería

- Aritmética entera comprobada y código Rust sin `unsafe`.
- IDs y digests BLAKE3 deterministas.
- Reserva segregada, journal y reconciliación por activo.
- Stress independiente de tesorería y agregación multi-activo.
- 11 pruebas Rust y 13 pruebas Node.
- CI Linux/Windows, formato, Clippy estricto y lockfiles.
- Promoción verificable entre `main`, `production`, tag y release.

## Licencia

Distribuido bajo los términos de [MIT](LICENSE).
