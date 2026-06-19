workspace "Deployment" {
  !identifiers hierarchical
  model {
    system = softwareSystem "System" {
      api = container "API" "Backend" "Rust"
    }
    production = deploymentEnvironment "Production" {
      blue = deploymentGroup "Blue"
      primary = deploymentNode "Primary" "Main host" "Linux" 2 "Host" {
        instances 3
        nested = deploymentNode "Nested" {
          gateway = infrastructureNode "Gateway" "Proxy" "nginx" "Edge"
          systemInstance = softwareSystemInstance system production.blue "Primary" {
            healthCheck "System" "https://system.test/health" 60 5
          }
          apiInstance = containerInstance system.api production.blue "Primary" {
            healthCheck "API" "https://api.test/health" 30 3
          }
        }
      }
      secondary = deploymentNode "Secondary"
      production.primary -> production.secondary "Replicates"
    }
  }
}
