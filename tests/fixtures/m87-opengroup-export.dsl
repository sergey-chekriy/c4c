workspace "M87 Open Group Export" {
  model {
    archimate {
      actor = businessActor "Operator"
      app = applicationComponent "Trading app"
      service = applicationService "Trading service"
      data = dataObject "Order data"
      accepted = andJunction "Accepted order"

      actor -> app "uses" {
        type AssociationRelationship
      }
      app -> service "publishes" {
        type FlowRelationship
      }
      app -> data "reads" {
        type AccessRelationship
        access read
      }
      service -> accepted "accepted" {
        type FlowRelationship
      }
      accepted -> data "stores" {
        type FlowRelationship
      }
    }
  }

  views {
    archimateView opengroup {
      viewpoint applicationCooperation
      include *
    }
  }
}
