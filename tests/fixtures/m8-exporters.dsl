workspace "Milestone 8 <Exporters> & Exchange" "Deterministic local formats" {
  !identifiers hierarchical
  model {
    user = person "User <Admin> & Owner" "Exercises exporter escaping"
    system = softwareSystem "Orders & Payments" "Primary application" {
      api = container "API <Gateway>" "Backend API" "Rust" {
        handler = component "Order Handler" "Handles orders" "Rust"
      }
    }
    external = softwareSystem "External Ledger"
    user -> system.api "Uses <securely> & safely" "HTTPS"
    system.api -> external "Posts entries" "JSON"
    system.api.handler -> external "Posts entries" "JSON"
    production = deploymentEnvironment "Production" {
      node = deploymentNode "Primary Node" "Local host" "Linux" {
        apiInstance = containerInstance system.api
      }
    }
  }
  views {
    systemLandscape landscape {
      include *
    }
    systemContext system context {
      include *
    }
    container system containers {
      include *
    }
    component system.api components {
      include *
    }
    dynamic system flow {
      user -> system.api "Open orders"
      system.api -> external "Post entry"
    }
    deployment system production deployment {
      include *
    }
  }
}
