import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Kestrel',
  description: 'The Kestrel Language',
  cleanUrls: true,
  themeConfig: {
    nav: [
      { text: 'Guide', link: '/getting-started/' },
      { text: 'Tour', link: '/tour/' },
      { text: 'Reference', link: '/reference/' },
    ],
    sidebar: [
      {
        text: 'Getting Started',
        collapsed: false,
        items: [
          { text: 'Overview', link: '/getting-started/' },
          { text: 'Installation', link: '/getting-started/installation' },
          { text: 'Hello, World', link: '/getting-started/hello-world' },
          { text: 'Flock', link: '/getting-started/flock' },
          { text: 'Kestrel Skill', link: '/getting-started/kestrel-skill' },
          { text: 'Install the LSP Extension', link: '/getting-started/lsp-extension' },
        ],
      },
      {
        text: 'A Tour of Kestrel',
        collapsed: false,
        items: [
          { text: 'Overview', link: '/tour/' },
          { text: 'Text Adventure', link: '/tour/text-adventure' },
          { text: 'Wizard Duel', link: '/tour/wizard-duel' },
          { text: 'Turtle Graphics', link: '/tour/turtle-graphics' },
        ],
      },
      {
        text: 'Language',
        collapsed: false,
        items: [
          { text: 'Values & Variables', link: '/values-and-variables' },
          {
            text: 'Functions',
            link: '/functions/',
            collapsed: true,
            items: [
              { text: 'Access Modes', link: '/functions/access-modes' },
              { text: 'Methods', link: '/functions/methods' },
              { text: 'Closures', link: '/functions/closures' },
              { text: 'Operator Overloading', link: '/functions/operator-overloading' },
            ],
          },
          { text: 'Control Flow', link: '/control-flow' },
          {
            text: 'Collections',
            link: '/collections/',
            collapsed: true,
            items: [
              { text: 'Arrays', link: '/collections/arrays' },
              { text: 'Dictionaries', link: '/collections/dictionaries' },
              { text: 'Sets', link: '/collections/sets' },
              { text: 'Tuples', link: '/collections/tuples' },
              { text: 'Iterators', link: '/collections/iterators' },
            ],
          },
          {
            text: 'Structs',
            link: '/structs/',
            collapsed: true,
            items: [
              { text: 'Fields', link: '/structs/fields' },
              { text: 'Methods', link: '/structs/methods' },
              { text: 'Initializers', link: '/structs/initializers' },
              { text: 'Deinitializers', link: '/structs/deinitializers' },
              { text: 'Computed Variables', link: '/structs/computed-variables' },
              { text: 'Subscripts', link: '/structs/subscripts' },
            ],
          },
          {
            text: 'Enums',
            link: '/enums/',
            collapsed: true,
            items: [
              { text: 'Pattern Matching', link: '/enums/pattern-matching' },
            ],
          },
          {
            text: 'Error Handling',
            link: '/error-handling/',
            collapsed: true,
            items: [
              { text: 'Optional', link: '/error-handling/optional' },
              { text: 'Result', link: '/error-handling/result' },
            ],
          },
          {
            text: 'Protocols',
            link: '/protocols/',
            collapsed: true,
            items: [
              { text: 'Defining', link: '/protocols/defining' },
              { text: 'Conformance', link: '/protocols/conformance' },
              { text: 'Default Methods', link: '/protocols/default-methods' },
              { text: 'Inheritance Rules', link: '/protocols/inheritance-rules' },
              { text: 'Extending', link: '/protocols/extending' },
            ],
          },
          {
            text: 'Generics',
            link: '/generics/',
            collapsed: true,
            items: [
              { text: 'Where Clauses', link: '/generics/where-clauses' },
              { text: 'Associated Types', link: '/generics/associated-types' },
            ],
          },
          { text: 'Extending Types', link: '/extending-types' },
          { text: 'Organization', link: '/organization' },
          { text: 'FFI', link: '/ffi' },
        ],
      },
      {
        text: 'Concepts',
        collapsed: false,
        items: [
          { text: 'Overview', link: '/concepts/' },
          { text: 'Type Inference', link: '/concepts/type-inference' },
          { text: 'Memory Model', link: '/concepts/memory-model' },
        ],
      },
      {
        text: 'Tooling',
        collapsed: false,
        items: [
          { text: 'Overview', link: '/tooling/' },
          { text: 'Flock', link: '/tooling/flock' },
          { text: 'Kestrel LSP', link: '/tooling/kestrel-lsp' },
          { text: 'Jessup', link: '/tooling/jessup' },
        ],
      },
      {
        text: 'Reference',
        collapsed: false,
        items: [
          { text: 'Overview', link: '/reference/' },
          { text: 'Diagnostics', link: '/reference/diagnostics' },
          { text: 'Stdlib', link: '/reference/stdlib' },
          { text: 'Operators', link: '/reference/operators' },
          { text: 'Builtins', link: '/reference/builtins' },
        ],
      },
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/' },
    ],
    search: {
      provider: 'local',
    },
    outline: {
      level: [2, 3],
    },
  },
})
