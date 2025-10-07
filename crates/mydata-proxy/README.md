## Description 

This crate contains 2 components 

### 1. `mydata-proxy`

#### Proxy and Processor: [location](./src/main.rs)
A public-facing HTTP proxy server that accepts metrics packets. It:

- Authenticates requests using bearer tokens (securely encrypted and stored internally)
- **Encodes the metrics into a Protobuf format**
- Relays them to an **Alloy sidecar container**, which forwards the data to the **Mimir metrics cluster**
- usage: `mydata-proxy --config=mydata-proxy.yaml --bearer-tokens-path=bearer-tokens.yaml`
    - [sample config file](../../docker/mydata-proxy/local-test/mydata-proxy.yaml)
    - [sample bearer token file](../../docker/mydata-proxy/local-test/bearer-tokens.yaml)

#### Client Library: [location](./src/client.rs)
A reusable library that allows clients to:

- Push metrics to the `mydata-proxy`
- Authenticate via bearer tokens
- please refer to Sample Client Setup for more details.

---

### 2. Sample Client Setup: [location](../../docker/mydata-proxy/local-test/metrics-generator/src/main.rs)

#### Metrics Generator
- A sample application that uses the client library to generate and push metrics.

#### MyData-Proxy Instance
- Receives, authenticates, and processes metrics.

#### Alloy Sidecar
- Collects histogram metrics from mydata-proxy instance due to lack of support of native histogram in prometheus rust library.

#### Mimir Cluster
- Stores the incoming metrics.

#### Grafana
- Visualizes metrics data stored in Mimir.

---

**Data Flow Summary:**  
`Auth → Encode → Relay → Store → Visualize`


## Test plan 

local end-to-end testing
- `cd docker/mydata-proxy/local-test`
- `docker compose up --build`
- navigate to `localhost:3000` to access grafana

## if provided with incorrect Bearer token
```
mydata-proxy  | 2025-07-18T21:51:50.471395Z  INFO tower_http::trace::on_response: finished processing request latency=0.000157083 s status=401
mydata-proxy  | 2025-07-18T21:51:55.462979Z  INFO mydata_proxy::middleware: auth_header: "Bearer 1234567890"
mydata-proxy  | 2025-07-18T21:51:55.462995Z  INFO mydata_proxy::allowers: Rejected Bearer Token: "1234567890"
mydata-proxy  | 2025-07-18T21:51:55.462996Z  INFO mydata_proxy::middleware: invalid token, rejecting request
```

## with correct Bearer token, it shows the corresponding name of the token
```
mydata-proxy  | 2025-07-18T21:55:17.199397Z  INFO tower_http::trace::on_response: finished processing request latency=0.001276666 s status=200
mydata-proxy  | 2025-07-18T21:55:22.193788Z  INFO mydata_proxy::middleware: auth_header: "Bearer abcdefghijklmnopqrstuvwxyz"
mydata-proxy  | 2025-07-18T21:55:22.193805Z  INFO mydata_proxy::allowers: Accepted Request from: "sample-token"
```

the `metrics-generator` creates some `int_gauge`, `int_gauge_vec` and `histogram` metrics
