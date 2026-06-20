!constant WORKSPACE_NAME "Milestone 6 Workspace"
workspace "${WORKSPACE_NAME}" {
  model {
    !include m6-parts
    !include m6-nested.dsl
  }
  views {
    systemLandscape expressions {
      include element.tag==Internal || element.tag==External
      exclude element.tag==Deprecated && element.type==Container
    }
  }
}
