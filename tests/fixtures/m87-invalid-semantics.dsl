workspace "M87 Invalid ArchiMate Semantics" {
  model {
    archimate {
      actor = businessActor "Operator"
      app = applicationComponent "Trading app"
      data = dataObject "Order data"
      badGoal = goal "Improve settlement"
      badObject = businessObject "Business document"
      runtime = node "Runtime"

      data -> app "assigned from passive" {
        type AssignmentRelationship
      }
      app -> actor "reads actor" {
        type AccessRelationship
        access read
      }
      badGoal -> data "flows into data" {
        type FlowRelationship
      }
      badObject -> app "realizes app" {
        type RealizationRelationship
      }
      app -> actor "specializes actor" {
        type SpecializationRelationship
      }
    }
  }

  views {
    archimateView wrong-motivation {
      viewpoint motivation
      include app runtime
    }
  }
}
