workspace "M85 ArchiMate Views Layout" {
  model {
    archimate {
      operator = businessActor "Operator"
      onboarding = businessProcess "Client Onboarding"
      apiService = applicationService "Trading API Service"
      portal = applicationComponent "Client Portal"
      gateway = applicationComponent "Internal API Gateway" {
        background #008e00
        color #ffffff
      }
      matcher = applicationFunction "Order Matcher"
      ledger = dataObject "Ledger Entry"
      accepted = andJunction "Accepted Order"
      audit = applicationComponent "Audit Trail"

      compliance = stakeholder "Compliance"
      regulation = driver "Regulatory pressure"
      traceabilityGoal = goal "Traceable settlement"
      auditRequirement = requirement "Every order is auditable"
      tradingCapability = capability "Digital asset trading"

      techService = technologyService "Runtime Service"
      runtime = node "Primary Runtime Node"
      gatewayArtifact = artifact "Gateway Artifact"
      rack = equipment "Rack A"

      operator -> onboarding "starts onboarding" {
        type TriggeringRelationship
      }
      onboarding -> apiService "requests account activation" {
        type ServingRelationship
      }
      portal -> gateway "Calls trading gateway over secure API with customer and order context" {
        type FlowRelationship
        color #00aa00
      }
      gateway -> accepted "validates order" {
        type FlowRelationship
      }
      accepted -> matcher "routes accepted order" {
        type TriggeringRelationship
      }
      matcher -> ledger "writes ledger entry" {
        type AccessRelationship
        access write
      }
      gateway -> ledger "reads latest ledger state" {
        type AccessRelationship
        access read
      }
      audit -> ledger "reads and writes audit snapshots" {
        type AccessRelationship
        access readWrite
      }
      apiService -> operator "serves operator workflow" {
        type ServingRelationship
      }

      compliance -> regulation "raises" {
        type InfluenceRelationship
      }
      regulation -> traceabilityGoal "motivates" {
        type InfluenceRelationship
      }
      traceabilityGoal -> auditRequirement "is refined by" {
        type RealizationRelationship
      }
      tradingCapability -> gateway "realized by" {
        type RealizationRelationship
      }

      techService -> runtime "is served by" {
        type ServingRelationship
      }
      runtime -> gatewayArtifact "deploys" {
        type AssignmentRelationship
      }
      rack -> runtime "contains" {
        type CompositionRelationship
      }
    }
  }

  views {
    archimateView m85-app {
      viewpoint applicationCooperation
      include operator onboarding apiService portal gateway matcher ledger accepted audit
      title "M85 application cooperation"

      object gateway {
        x 1000
        y 320
        width 220
        height 90
        background #008e00
      }
    }

    archimateView m85-motivation {
      viewpoint motivation
      include compliance regulation traceabilityGoal auditRequirement tradingCapability gateway
      title "M85 motivation"
    }

    archimateView m85-technology {
      viewpoint technology
      include techService runtime gatewayArtifact rack
      title "M85 technology"

      object runtime {
        x 160
        y 380
        width 220
        height 90
      }
    }
  }
}
