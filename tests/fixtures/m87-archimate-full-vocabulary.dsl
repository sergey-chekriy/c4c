workspace "M87 ArchiMate Full Vocabulary" {
  model {
    archimate {
      m87Stakeholder = stakeholder "Regulator"
      m87Driver = driver "Market demand"
      m87Assessment = assessment "Risk assessment"
      m87Goal = goal "Reliable trading"
      m87Outcome = outcome "Completed trade"
      m87Principle = principle "Auditability"
      m87Requirement = requirement "Trace every order"
      m87Constraint = constraint "Local processing"
      m87Meaning = meaning "Order meaning"
      m87Value = value "Customer value"

      m87Resource = resource "Trading desk"
      m87Capability = capability "Digital trading"
      m87ValueStream = valueStream "Order to settlement"
      m87Course = courseOfAction "Automate settlement"

      m87Actor = businessActor "Operator"
      m87Role = businessRole "Trader"
      m87Collaboration = businessCollaboration "Trading desk collaboration"
      m87BusinessInterface = businessInterface "Trading desk portal"
      m87Process = businessProcess "Place order"
      m87BusinessFunction = businessFunction "Order management"
      m87Interaction = businessInteraction "Approve order"
      m87Event = businessEvent "Order received"
      m87BusinessService = businessService "Trading service"
      m87BusinessObject = businessObject "Order"
      m87Contract = contract "Trading agreement"
      m87Representation = representation "Order form"
      m87Product = product "Trading product"

      m87App = applicationComponent "Trading application"
      m87AppCollaboration = applicationCollaboration "Application collaboration"
      m87AppInterface = applicationInterface "REST API"
      m87AppFunction = applicationFunction "Match order"
      m87AppInteraction = applicationInteraction "Quote interaction"
      m87AppProcess = applicationProcess "Order workflow"
      m87AppEvent = applicationEvent "Order submitted"
      m87AppService = applicationService "Order service"
      m87Data = dataObject "Order data"

      m87Runtime = node "Primary node"
      m87Device = device "Server"
      m87SystemSoftware = systemSoftware "Linux"
      m87TechCollaboration = technologyCollaboration "Cluster"
      m87TechInterface = technologyInterface "HTTPS endpoint"
      m87Path = path "Network path"
      m87Network = communicationNetwork "Private network"
      m87TechFunction = technologyFunction "Store data"
      m87TechProcess = technologyProcess "Deploy artifact"
      m87TechInteraction = technologyInteraction "Node coordination"
      m87TechEvent = technologyEvent "Deployment started"
      m87TechService = technologyService "Runtime service"
      m87Artifact = artifact "Trading artifact"

      m87Equipment = equipment "Rack"
      m87Facility = facility "Data center"
      m87DistributionNetwork = distributionNetwork "Power distribution"
      m87Material = material "Hardware material"

      m87Work = workPackage "Trading release"
      m87Deliverable = deliverable "Release package"
      m87ImplEvent = implementationEvent "Go live"
      m87Plateau = plateau "Target plateau"
      m87Gap = gap "Migration gap"

      m87Grouping = grouping "Trading context"
      m87Location = location "Reykjavik"
      m87Junction = junction "Generic junction"
      m87AndJoin = andJunction "Order split"
      m87OrJoin = orJunction "Order merge"

      m87Actor -> m87Process "performs" {
        type AssignmentRelationship
      }
      m87AppFunction -> m87Data "reads" {
        type AccessRelationship
        access read
      }
      m87App -> m87AppService "contains" {
        type CompositionRelationship
      }
      m87Grouping -> m87Location "groups" {
        type AggregationRelationship
      }
      m87Process -> m87BusinessService "realizes" {
        type RealizationRelationship
      }
      m87AppService -> m87App "serves" {
        type ServingRelationship
      }
      m87Goal -> m87Requirement "influences" {
        type InfluenceRelationship
      }
      m87AppEvent -> m87AppProcess "triggers" {
        type TriggeringRelationship
      }
      m87AppProcess -> m87AppService "flows" {
        type FlowRelationship
      }
      m87App -> m87AppCollaboration "specializes" {
        type SpecializationRelationship
      }
      m87Actor -> m87App "uses" {
        type AssociationRelationship
      }
      m87AppProcess -> m87AndJoin "accepted" {
        type FlowRelationship
      }
      m87AndJoin -> m87OrJoin "routed" {
        type FlowRelationship
      }
      m87OrJoin -> m87Data "stored" {
        type FlowRelationship
      }
    }
  }

  views {
    archimateView m87-application {
      viewpoint applicationStructure
      include m87App m87AppCollaboration m87AppInterface m87AppFunction m87AppInteraction m87AppProcess m87AppEvent m87AppService m87Data m87AndJoin m87OrJoin
    }

    archimateView m87-business {
      viewpoint businessProcess
      include m87Actor m87Role m87Collaboration m87BusinessInterface m87Process m87BusinessFunction m87Interaction m87Event m87BusinessService m87BusinessObject m87Contract m87Representation m87Product
    }

    archimateView m87-capability {
      viewpoint capabilityMap
      include m87Resource m87Capability m87ValueStream m87Course
    }

    archimateView m87-project {
      viewpoint project
      include m87Work m87Deliverable m87ImplEvent m87Plateau m87Gap
    }

    archimateView m87-technology {
      viewpoint technology
      include m87Runtime m87Device m87SystemSoftware m87TechCollaboration m87TechInterface m87Path m87Network m87TechFunction m87TechProcess m87TechInteraction m87TechEvent m87TechService m87Artifact m87Equipment m87Facility m87DistributionNetwork m87Material
    }

    archimateView m87-motivation {
      viewpoint motivation
      include m87Stakeholder m87Driver m87Assessment m87Goal m87Outcome m87Principle m87Requirement m87Constraint m87Meaning m87Value
    }
  }
}
