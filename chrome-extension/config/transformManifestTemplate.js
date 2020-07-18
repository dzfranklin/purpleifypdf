"use strict";

const packageJson = require('../package.json');

const REPLACEMENTS = {
    "%VERSION%": packageJson.version,
    "%BACKGROUND_PAGE%": "background.html"
}

function transformManifestTemplate(templateBuffer) {
    // called inside of webpack.config.js
    let template = templateBuffer.toString();
    for (const key in REPLACEMENTS) {
        template = template.replace(new RegExp(key, 'g'), REPLACEMENTS[key]);
    }
    return template;
}

module.exports = transformManifestTemplate;