// aegis-highlight.js
hljs.registerLanguage('aegis', function(hljs) {
    return {
        name: 'Aegis',
        aliases: ['aeg'], // Permet d'utiliser ```aeg aussi
        keywords: {
            // Mots-clés de contrôle et structure
            keyword: 'var func class namespace import return if else while for switch case default try catch throw new print input break continue',
            // Valeurs littérales
            literal: 'true false null',
            // Variables spéciales ou modules natifs courants
            built_in: 'this System File Http Json Math Random Test Assert Base64 Hash Date Process Path',
            // Tes types pour le typage graduel
            type: 'int float string bool list dict func'
        },
        contains: [
            hljs.QUOTE_STRING_MODE,       // "string"
            hljs.C_LINE_COMMENT_MODE,     // // comment
            hljs.C_BLOCK_COMMENT_MODE,    // /* comment */
            hljs.NUMBER_MODE,             // 123, 12.5
            
            // Coloration spécifique pour la définition de fonctions (func nom)
            {
                className: 'function',
                beginKeywords: 'func', 
                end: /\{/, 
                excludeEnd: true,
                contains: [
                    hljs.TITLE_MODE,
                    {
                        className: 'params',
                        begin: /\(/, end: /\)/
                    }
                ]
            },
            // Coloration spécifique pour les classes et namespaces
            {
                className: 'class',
                beginKeywords: 'class namespace', 
                end: /\{/, 
                excludeEnd: true,
                contains: [hljs.TITLE_MODE]
            },
            // Décorateurs (@logger)
            {
                className: 'meta',
                begin: '@[a-zA-Z_][a-zA-Z0-9_]*'
            }
        ]
    };
});

// Relance la coloration si le script est chargé après le DOM
hljs.initHighlightingOnLoad();
