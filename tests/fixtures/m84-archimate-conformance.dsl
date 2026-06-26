workspace "M84 ArchiMate Conformance" {
  model {
    archimate {
      operator = businessActor "Operator"
      onboarding = businessProcess "Customer onboarding"
      customerRecord = businessObject "Customer record"

      dex = applicationComponent "DEX Application" {
        background #008e00
        color #ffffff
      }
      gateway = applicationComponent "Internal API Gateway"
      api = applicationService "Trading API"
      matcher = applicationFunction "Order Matcher"
      ledger = dataObject "Ledger Entry"

      runtime = node "Primary Node"
      dexArtifact = artifact "DEX Artifact"
      rack = equipment "Rack"

      riskGoal = goal "Reduce settlement risk"
      tradingCapability = capability "Digital asset trading"
      delivery = workPackage "DEX delivery"
      platform = deliverable "Trading platform"
      andJoin = andJunction "Order accepted"
      orJoin = orJunction "Settlement alternative"

      rAssignment = operator -> onboarding "performs" {
        type AssignmentRelationship
      }
      onboarding -> customerRecord "reads" {
        type AccessRelationship
        access read
      }
      gateway -> api "serves" {
        type ServingRelationship
      }
      gateway -> matcher "triggers" {
        type TriggeringRelationship
      }
      matcher -> ledger "writes" {
        type AccessRelationship
        access write
      }
      matcher -> api "publishes orders" {
        type FlowRelationship
        color #00aa00
      }
      runtime -> dexArtifact "hosts" {
        type AssignmentRelationship
      }
      tradingCapability -> dex "realized by" {
        type RealizationRelationship
      }
      riskGoal -> tradingCapability "influences" {
        type InfluenceRelationship
      }
      delivery -> platform "produces" {
        type RealizationRelationship
      }
      dex -> gateway "contains" {
        type CompositionRelationship
      }
      gateway -> dex "part of platform" {
        type AggregationRelationship
      }
      api -> andJoin "accepted by" {
        type FlowRelationship
      }
      andJoin -> orJoin "routes to" {
        type FlowRelationship
      }
      orJoin -> ledger "settles" {
        type FlowRelationship
      }
      gateway -> operator "notifies" {
        type AssociationRelationship
      }
    }
  }

  views {
    archimateView m84-app {
      viewpoint applicationCooperation
      include operator onboarding customerRecord dex gateway api matcher ledger andJoin orJoin
      title "M84 application cooperation"

      object gateway {
        x 300
        y 120
        width 180
        height 80
        background #008e00
      }
      object ledger {
        x 600
        y 120
        width 180
        height 80
      }
    }

    archimateView m84-tech {
      viewpoint technology
      include runtime dexArtifact rack
      title "M84 technology"
    }

    archimateView m84-motivation {
      viewpoint motivation
      include riskGoal tradingCapability
      title "M84 motivation"
    }
  }
}
