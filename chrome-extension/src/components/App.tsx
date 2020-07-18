import React, { useState, useEffect, useMemo } from 'react';
import { getApi } from '../utils/backgroundApi';
import useStorage from '../hooks/useStorage';
import { Quality, qualityFromString } from '../utils/quality';
import HexColor from '../utils/hexcolor';

import Progress, { ProgressProps } from './Progress';
import ErrorMsg, { ErrorMsgProps, formatSource } from './ErrorMsg';
import Controls from './Controls';
import PdfDisplay, { PdfMetadata, PdfPage } from './PdfDisplay';

export default function App() {
    let [errors, setErrors] = useState<ErrorMsgProps[]>([]);
    const appendError = (err: ErrorMsgProps) => setErrors(prevErrors => prevErrors.concat(err));
    const prependError = (err: ErrorMsgProps) => setErrors(prevErrors => [err].concat(prevErrors));

    let [state, setState] = useState<{ progress: ProgressProps } | { metadata: PdfMetadata }>({
        progress: {
            label: "Loading",
            percent: null
        }
    });

    let windowTitle = "Loading...";
    if ('metadata' in state) {
        windowTitle = state.metadata.title;
    }
    useEffect(() => {
        document.title = windowTitle;
    }, [windowTitle]);

    let [pages, setPages] = useState<PdfPage[]>([]);

    // const [backgroundColor, setBackgroundColor] = useStorage('setting-background-color',
    //     new HexColor('#e261ff'),
    //     { load: s => new HexColor(s), stringify: v => v.css }
    // );
    // const [quality, setQuality] = useStorage('setting-quality',
    //     Quality.High,
    //     { load: qualityFromString, stringify: v => v.toString() }
    // );
    const [backgroundColor, setBackgroundColor] = useState(new HexColor('#e261ff'));
    const [quality, setQuality] = useState(Quality.High);

    const requestId = useMemo(() => new URLSearchParams(window.location.search).get("request_id"), []);

    useEffect(() => {
        transform(
            requestId,
            quality,
            backgroundColor,
            (metadata: PdfMetadata) => setState({ metadata }),
            (page: PdfPage) => setPages(prev => prev.concat(page))
        ).catch(err => appendError(err.props));
    }, [requestId, quality, backgroundColor]);

    if (errors.length !== 0) {
        return (<>
            <div className="max-w-xl m-5 bg-blue-200 border-l-4 border-blue-600 text-blue-800 p-6 pt-8">
                <h1 className="text-xl font-bold pb-3">Give up?</h1>
                <p>Temporarily turn off PDF transformation for this tab.</p>
                <button
                    className="rounded-md py-2 px-4 mt-4 bg-blue-700 text-blue-100"
                    onClick={() => {
                        getApi()
                            .then(api => api.fallback(requestId))
                            .then(url => window.location.href = url)
                            .catch(reason => prependError({
                                title: 'Failed to turn off transformation',
                                children: (<>
                                    <p>While trying to turn off other errors ocurred</p>
                                    <p>You may want to disable this extension in the chrome extension settings.</p>
                                    {formatSource(reason)}
                                </>)
                            }));
                    }}>Turn off</button>
            </div>

            {errors.map((error, idx) => (<ErrorMsg title={error.title} key={idx}>{error.children}</ErrorMsg>))}
        </>);
    } else if ('progress' in state) {
        return (<Progress {...state.progress}></Progress>);
    } else {
        return (<>
            <PdfDisplay
                metadata={state.metadata}
                pages={pages}
            ></PdfDisplay>

            <Controls
                quality={quality} setQuality={setQuality}
                backgroundColor={backgroundColor} setBackgroundColor={setBackgroundColor}
            ></Controls>
        </>);
    }
}

class DisplayableError extends Error {
    public props: ErrorMsgProps;

    constructor(props: ErrorMsgProps) {
        super();
        this.props = props;
        this.name = "TransformError";
    }
}

async function transform(
    requestId: string | null,
    quality: Quality,
    backgroundColor: HexColor,
    setMetadata: (meta: PdfMetadata) => void,
    appendPage: (page: PdfPage) => void): Promise<void> {

    let api;
    try {
        api = await getApi();
    } catch (err) {
        throw new DisplayableError({
            title: 'Error opening communication with background page',
            children: formatSource(err)
        });
    }

    let results;
    try {
        if (!requestId) {
            throw new DisplayableError({
                title: "Invalid URL",
                children: "The URL is missing required parameters."
            });
        }
        results = api.transform(requestId, quality, backgroundColor);
    } catch (err) {
        throw new DisplayableError({
            title: 'Error transforming: ' + err.type.title,
            children: err.type.children.concat(formatSource(err.source))
        });
    }

    for await (const result of results) {
        if (result.isMetadata) {
            const { originalTitle, pageCount } = result;
            setMetadata({
                title: originalTitle,
                pageCount
            });
        } else {
            appendPage(result);
        }
    }
};