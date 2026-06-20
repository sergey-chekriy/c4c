const ci = word => new RegExp(word.split('').map(char => {
  const lower = char.toLowerCase();
  const upper = char.toUpperCase();
  return lower === upper ? `\\${char}` : `[${lower}${upper}]`;
}).join(''));

const kw = ($, word) => alias(token(prec(2, ci(word))), $.keyword);

module.exports = grammar({
  name: 'structurizr_dsl',

  extras: $ => [/[ \t\r]/, $.comment, $.line_continuation],

  rules: {
    source_file: $ => seq(repeat($._newline), $.workspace),

    workspace: $ => seq(
      kw($, 'workspace'),
      choice(
        seq(kw($, 'extends'), $.value),
        seq(optional($.value), optional($.value)),
      ),
      $._workspace_block,
    ),

    _workspace_block: $ => seq(
      '{', $._newline,
      repeat(choice($._newline, $._workspace_statement)),
      '}', repeat($._newline),
    ),

    _workspace_statement: $ => choice(
      $.model,
      $.views,
      $.workspace_property,
      $.property_block,
      $.configuration_block,
      $.directive,
    ),

    workspace_property: $ => seq(
      choice(kw($, 'name'), kw($, 'description')),
      $.value,
      $._newline,
    ),

    model: $ => seq(kw($, 'model'), $._model_block),

    _model_block: $ => seq(
      '{', $._newline,
      repeat(choice($._newline, $._model_statement)),
      '}', $._newline,
    ),

    _model_statement: $ => choice(
      $.element_declaration,
      $.relationship,
      $.removed_relationship,
      $.enterprise,
      $.group,
      $.archetypes_block,
      $.property_statement,
      $.property_block,
      $.perspectives_block,
      $.instance_of_statement,
      $.health_check_statement,
      $.directive,
    ),

    element_declaration: $ => seq(
      optional(seq(field('id', $.identifier), '=')),
      field('kind', choice(
        kw($, 'person'),
        kw($, 'softwareSystem'),
        kw($, 'container'),
        kw($, 'component'),
        kw($, 'deploymentEnvironment'),
        kw($, 'deploymentGroup'),
        kw($, 'deploymentNode'),
        kw($, 'infrastructureNode'),
        kw($, 'softwareSystemInstance'),
        kw($, 'containerInstance'),
        kw($, 'element'),
      )),
      repeat1($.value),
      choice($._newline, $._model_block),
    ),

    relationship: $ => seq(
      field('source', $.identifier),
      field('operator', '->'),
      field('destination', $.identifier),
      repeat($.value),
      choice($._newline, $._relationship_block),
    ),

    removed_relationship: $ => seq(
      field('source', $.identifier),
      field('operator', '-/>'),
      field('destination', $.identifier),
      optional($.value),
      $._newline,
    ),

    _relationship_block: $ => seq(
      '{', $._newline,
      repeat(choice(
        $._newline,
        $.property_statement,
        $.property_block,
        $.perspectives_block,
        $.directive,
      )),
      '}', $._newline,
    ),

    enterprise: $ => seq(kw($, 'enterprise'), $.value, $._model_block),
    group: $ => seq(kw($, 'group'), $.value, $._model_block),
    archetypes_block: $ => seq(kw($, 'archetypes'), $._generic_block),
    configuration_block: $ => seq(kw($, 'configuration'), $._generic_block),

    property_statement: $ => seq(
      choice(
        kw($, 'description'),
        kw($, 'technology'),
        kw($, 'tag'),
        kw($, 'tags'),
        kw($, 'url'),
        kw($, 'instances'),
      ),
      $.value,
      $._newline,
    ),

    instance_of_statement: $ => seq(kw($, 'instanceOf'), $.identifier, $._newline),

    health_check_statement: $ => seq(
      kw($, 'healthCheck'),
      $.value,
      $.value,
      optional($.value),
      optional($.value),
      $._newline,
    ),

    property_block: $ => seq(kw($, 'properties'), $._property_entries),
    perspectives_block: $ => seq(kw($, 'perspectives'), $._property_entries),

    _property_entries: $ => seq(
      '{', $._newline,
      repeat(choice($._newline, $.property_entry)),
      '}', $._newline,
    ),

    property_entry: $ => seq($.value, $.value, $._newline),

    directive: $ => seq(
      choice(
        kw($, '!identifiers'),
        kw($, '!impliedRelationships'),
        kw($, '!docs'),
        kw($, '!adrs'),
        kw($, '!extend'),
        kw($, '!ref'),
        kw($, '!element'),
        kw($, '!elements'),
        kw($, '!relationship'),
        kw($, '!relationships'),
        kw($, '!components'),
        kw($, '!script'),
        kw($, '!plugin'),
      ),
      repeat(choice($.value, '*')),
      choice($._newline, $._generic_block),
    ),

    views: $ => seq(kw($, 'views'), $._views_block),

    _views_block: $ => seq(
      '{', $._newline,
      repeat(choice($._newline, $._view_statement)),
      '}', $._newline,
    ),

    _view_statement: $ => choice(
      $.system_landscape_view,
      $.system_context_view,
      $.container_view,
      $.component_view,
      $.filtered_view,
      $.dynamic_view,
      $.deployment_view,
      $.custom_view,
      $.image_view,
      $.property_block,
      $.styles_block,
      $.theme_statement,
      $.themes_statement,
      $.terminology_block,
    ),

    system_landscape_view: $ => seq(
      kw($, 'systemLandscape'),
      optional($.value), optional($.value),
      $._static_view_block,
    ),

    system_context_view: $ => seq(
      kw($, 'systemContext'),
      field('scope', $.identifier),
      optional($.value), optional($.value),
      $._static_view_block,
    ),

    container_view: $ => seq(
      kw($, 'container'),
      field('scope', $.identifier),
      optional($.value), optional($.value),
      $._static_view_block,
    ),

    component_view: $ => seq(
      kw($, 'component'),
      field('scope', $.identifier),
      optional($.value), optional($.value),
      $._static_view_block,
    ),

    filtered_view: $ => seq(
      kw($, 'filtered'),
      field('base', $.identifier),
      field('mode', choice(kw($, 'include'), kw($, 'exclude'))),
      field('tags', $.value),
      optional($.value), optional($.value),
      $._static_view_block,
    ),

    dynamic_view: $ => seq(
      kw($, 'dynamic'),
      field('scope', choice('*', $.identifier)),
      optional($.value), optional($.value),
      $._dynamic_view_block,
    ),

    deployment_view: $ => seq(
      kw($, 'deployment'),
      field('scope', choice('*', $.identifier)),
      field('environment', $.value),
      optional($.value), optional($.value),
      $._static_view_block,
    ),

    custom_view: $ => seq(
      kw($, 'custom'),
      optional($.value), optional($.value), optional($.value),
      $._static_view_block,
    ),

    image_view: $ => seq(
      kw($, 'image'),
      field('scope', choice('*', $.identifier)),
      optional($.value),
      $._image_view_block,
    ),

    _static_view_block: $ => seq(
      '{', $._newline,
      repeat(choice($._newline, $._common_view_statement)),
      '}', $._newline,
    ),

    _dynamic_view_block: $ => seq(
      '{', $._newline,
      repeat(choice($._newline, $._common_view_statement, $.dynamic_relationship)),
      '}', $._newline,
    ),

    _image_view_block: $ => seq(
      '{', $._newline,
      repeat(choice($._newline, $._common_view_statement, $.image_source)),
      '}', $._newline,
    ),

    _common_view_statement: $ => choice(
      $.include_statement,
      $.exclude_statement,
      $.auto_layout_statement,
      $.default_statement,
      $.animation,
      $.view_title_statement,
      $.view_description_statement,
      $.property_block,
    ),

    include_statement: $ => seq(kw($, 'include'), repeat1($._selector), $._newline),
    exclude_statement: $ => seq(kw($, 'exclude'), repeat1($._selector), $._newline),

    _selector: $ => choice(
      $.value,
      '*',
      '*?',
      '->',
      '=',
      '==',
      '!=',
      '&&',
      '||',
      '(',
      ')',
    ),

    auto_layout_statement: $ => seq(
      choice(kw($, 'autoLayout'), kw($, 'autolayout')),
      optional($.value), optional($.value), optional($.value),
      $._newline,
    ),
    default_statement: $ => seq(kw($, 'default'), $._newline),
    view_title_statement: $ => seq(kw($, 'title'), $.value, $._newline),
    view_description_statement: $ => seq(kw($, 'description'), $.value, $._newline),

    animation: $ => seq(
      kw($, 'animation'), '{', $._newline,
      repeat(choice($._newline, $.animation_step)),
      '}', $._newline,
    ),
    animation_step: $ => seq(repeat1($.value), $._newline),

    dynamic_relationship: $ => choice(
      seq(
        optional(field('order', $.order)),
        field('source', $.identifier),
        '->',
        field('destination', $.identifier),
        optional($.value), optional($.value),
        $._newline,
      ),
      seq(
        optional(field('order', $.order)),
        field('relationship', $.identifier),
        optional($.value),
        $._newline,
      ),
    ),

    image_source: $ => choice(
      seq(choice(kw($, 'plantuml'), kw($, 'mermaid'), kw($, 'image')), $.value, $._newline),
      seq(kw($, 'kroki'), $.value, $.value, $._newline),
    ),

    styles_block: $ => seq(kw($, 'styles'), $._generic_block),
    terminology_block: $ => seq(kw($, 'terminology'), $._generic_block),
    theme_statement: $ => seq(kw($, 'theme'), $.value, $._newline),
    themes_statement: $ => seq(kw($, 'themes'), repeat1($.value), $._newline),

    _generic_block: $ => seq(
      '{', $._newline,
      repeat(choice($._newline, $.generic_statement)),
      '}', $._newline,
    ),
    generic_statement: $ => seq(
      repeat1(choice($.value, '=', '*', '->', '-/>')),
      choice($._newline, $._generic_block),
    ),

    value: $ => choice($.identifier, $.string),
    order: _ => token(prec(3, /[0-9]+:/)),
    identifier: _ => token(prec(-1, /[A-Za-z0-9_.,:\/-]+/)),
    string: _ => /"([^"\\]|\\.)*"/,
    comment: _ => token(choice(/\/\/[^\n]*/, /#[^\n]*/)),
    line_continuation: _ => token(/\\\r?\n/),
    _newline: _ => /\n/,
  },
});
