workspace "Milestone 4 Deployment" {
  !identifiers hierarchical
  model {
    system = softwareSystem "System" {
      api = container "API"
    }
    other = softwareSystem "Other" {
      worker = container "Worker"
    }
    production = deploymentEnvironment "Production" {
      node = deploymentNode "Node" {
        proxy = infrastructureNode "Proxy"
        apiInstance = containerInstance system.api
        workerInstance = containerInstance other.worker
      }
    }
  }
  views {
    deployment * production allDeployment {
      default
      include *
    }
    deployment system "Production" systemDeployment {
      include *
      exclude production.node.proxy
      animation {
        production.node
        production.node.apiInstance
      }
    }
  }
}
