workspace "M88 ArchiMate 3.2 Full Vocabulary" {
  properties {
    archimateBaseline "3.2"
  }

  model {
    archimate {
      m88Stakeholder = stakeholder "Regulator"
      m88Driver = driver "Market demand"
      m88Assessment = assessment "Risk assessment"
      m88Goal = goal "Reliable trading"
      m88Outcome = outcome "Completed trade"
      m88Principle = principle "Auditability"
      m88Requirement = requirement "Trace every order"
      m88Constraint = constraint "Local processing"
      m88Meaning = meaning "Order meaning"
      m88Value = value "Customer value"

      m88Resource = resource "Trading desk"
      m88Capability = capability "Digital trading"
      m88ValueStream = valueStream "Order to settlement"
      m88Course = courseOfAction "Automate settlement"

      m88Actor = businessActor "Operator"
      m88Role = businessRole "Trader"
      m88Collaboration = businessCollaboration "Trading desk collaboration"
      m88BusinessInterface = businessInterface "Trading desk portal"
      m88Process = businessProcess "Place order"
      m88BusinessFunction = businessFunction "Order management"
      m88Interaction = businessInteraction "Approve order"
      m88Event = businessEvent "Order received"
      m88BusinessService = businessService "Trading service"
      m88BusinessObject = businessObject "Order"
      m88Contract = contract "Trading agreement"
      m88Representation = representation "Order form"
      m88Product = product "Trading product"

      m88App = applicationComponent "Trading application"
      m88AppCollaboration = applicationCollaboration "Application collaboration"
      m88AppInterface = applicationInterface "REST API"
      m88AppFunction = applicationFunction "Match order"
      m88AppInteraction = applicationInteraction "Quote interaction"
      m88AppProcess = applicationProcess "Order workflow"
      m88AppEvent = applicationEvent "Order submitted"
      m88AppService = applicationService "Order service"
      m88Data = dataObject "Order data"

      m88Runtime = node "Primary node"
      m88Device = device "Server"
      m88SystemSoftware = systemSoftware "Linux"
      m88TechCollaboration = technologyCollaboration "Cluster"
      m88TechInterface = technologyInterface "HTTPS endpoint"
      m88Path = path "Network path"
      m88Network = communicationNetwork "Private network"
      m88TechFunction = technologyFunction "Store data"
      m88TechProcess = technologyProcess "Deploy artifact"
      m88TechInteraction = technologyInteraction "Node coordination"
      m88TechEvent = technologyEvent "Deployment started"
      m88TechService = technologyService "Runtime service"
      m88Artifact = artifact "Trading artifact"

      m88Equipment = equipment "Rack"
      m88Facility = facility "Data center"
      m88DistributionNetwork = distributionNetwork "Power distribution"
      m88Material = material "Hardware material"

      m88Work = workPackage "Trading release"
      m88Deliverable = deliverable "Release package"
      m88ImplEvent = implementationEvent "Go live"
      m88Plateau = plateau "Target plateau"
      m88Gap = gap "Migration gap"

      m88Grouping = grouping "Trading context"
      m88Location = location "Reykjavik"
      m88Junction = junction "Generic junction"
      m88AndJoin = andJunction "Order split"
      m88OrJoin = orJunction "Order merge"

      m88Actor -> m88Process "performs" {
        type AssignmentRelationship
      }
      m88AppFunction -> m88Data "reads" {
        type AccessRelationship
        access read
      }
      m88App -> m88AppService "contains" {
        type CompositionRelationship
      }
      m88Grouping -> m88Location "groups" {
        type AggregationRelationship
      }
      m88Process -> m88BusinessService "realizes" {
        type RealizationRelationship
      }
      m88AppService -> m88App "serves" {
        type ServingRelationship
      }
      m88Goal -> m88Requirement "influences" {
        type InfluenceRelationship
      }
      m88AppEvent -> m88AppProcess "triggers" {
        type TriggeringRelationship
      }
      m88AppProcess -> m88AppService "flows" {
        type FlowRelationship
      }
      m88App -> m88AppCollaboration "specializes" {
        type SpecializationRelationship
      }
      m88Actor -> m88App "uses" {
        type AssociationRelationship
      }
      m88AppProcess -> m88AndJoin "accepted" {
        type FlowRelationship
      }
      m88AndJoin -> m88OrJoin "routed" {
        type FlowRelationship
      }
      m88OrJoin -> m88Data "stored" {
        type FlowRelationship
      }
    }
  }

  views {
    archimateView m88-application {
      viewpoint applicationStructure
      include m88App m88AppCollaboration m88AppInterface m88AppFunction m88AppInteraction m88AppProcess m88AppEvent m88AppService m88Data m88AndJoin m88OrJoin
    }

    archimateView m88-business {
      viewpoint businessProcess
      include m88Actor m88Role m88Collaboration m88BusinessInterface m88Process m88BusinessFunction m88Interaction m88Event m88BusinessService m88BusinessObject m88Contract m88Representation m88Product
    }

    archimateView m88-capability {
      viewpoint capabilityMap
      include m88Resource m88Capability m88ValueStream m88Course
    }

    archimateView m88-project {
      viewpoint project
      include m88Work m88Deliverable m88ImplEvent m88Plateau m88Gap
    }

    archimateView m88-technology {
      viewpoint technology
      include m88Runtime m88Device m88SystemSoftware m88TechCollaboration m88TechInterface m88Path m88Network m88TechFunction m88TechProcess m88TechInteraction m88TechEvent m88TechService m88Artifact m88Equipment m88Facility m88DistributionNetwork m88Material
    }

    archimateView m88-motivation {
      viewpoint motivation
      include m88Stakeholder m88Driver m88Assessment m88Goal m88Outcome m88Principle m88Requirement m88Constraint m88Meaning m88Value
    }
  }
}
