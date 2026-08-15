# Integración y observabilidad

El contrato de integración es la salida JSON del binario. El cliente JavaScript inicia el proceso sin shell, impone timeout y límite de buffer, valida la forma del reporte y propaga errores con contexto acotado.

## Frontera de integración

```mermaid
flowchart LR
    APP["Servicio consumidor"] -->|scenario + args| SDK["EmberlineClient"]
    SDK -->|spawn, no shell| BIN["emberline_dtl"]
    BIN -->|stdout JSON| SDK
    BIN -->|stderr acotado| SDK
    SDK -->|validated report| APP
    APP --> AUD["Audit store"]
    APP --> MET["Metrics"]
```

El path del binario debe configurarse de forma absoluta o resolver a un artefacto aprobado. No se aceptan argumentos construidos desde texto libre. En servicios concurrentes, cada invocación recibe un directorio temporal propio y límites de CPU/memoria del runtime.

## Contrato de errores

```mermaid
flowchart TD
    X["Invocation"] --> T{"Timeout"}
    T -->|sí| ET["PROCESS_TIMEOUT"]
    T -->|no| C{"Exit code = 0"}
    C -->|no| EP["PROCESS_FAILURE"]
    C -->|sí| J{"JSON válido"}
    J -->|no| EJ["INVALID_JSON"]
    J -->|sí| V{"Schema mínimo"}
    V -->|no| ES["INVALID_REPORT"]
    V -->|sí| OK["ProtocolReport"]
```

Los errores no deben registrar comandos de autenticación, secretos o datos personales. Para diagnóstico bastan código, versión del binario, duración, tamaño de salida, escenario permitido y hash del input.

## Trazabilidad

```mermaid
sequenceDiagram
    participant A as Application
    participant C as Client
    participant E as Engine
    participant O as Observability
    A->>C: request(trace_id)
    C->>E: deterministic input
    E-->>C: report(state_digest)
    C-->>A: validated output
    A->>O: duration + status + digest
    A->>O: policy_version + snapshot_version
```

Campos recomendados:

| Campo              | Tipo             | Cardinalidad     |
| ------------------ | ---------------- | ---------------- |
| `protocol_version` | label            | baja             |
| `scenario`         | label controlada | baja             |
| `asset`            | label permitida  | media            |
| `result_code`      | label            | baja             |
| `duration_ms`      | histograma       | n/a              |
| `state_digest`     | log estructurado | alta, no métrica |
| `route_id`         | traza/log        | alta, no métrica |

## Ejemplo de cliente

```js
import { EmberlineClient } from "../sdk/client.js";

const client = new EmberlineClient({
  binaryPath: process.env.EMBERLINE_BINARY,
  timeoutMs: 5_000,
  maxBufferBytes: 2 * 1024 * 1024,
});

const result = client.runScenario("normal");
if (!result.state_digest) throw new Error("Missing state digest");
```

La aplicación debe fijar una lista de escenarios permitidos. Para entradas dinámicas se recomienda añadir un canal estructurado versionado en vez de convertir propiedades externas en argumentos de terminal.
