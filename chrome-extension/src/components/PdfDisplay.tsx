import React from 'react';

export interface PdfDisplayProps {
    metadata: PdfMetadata,
    pages: PdfPage[]
}

export interface PdfMetadata {
    title: string,
    pageCount: number
}

export interface PdfPage {
    imageUrl: string,
    offset: number
}

export default function PdfDisplay(props: PdfDisplayProps): JSX.Element {
    // TODO: GC issue where bg will never clean up blobs
    return (<>
        <ul>
            {props.pages.map(({ imageUrl, offset }) => (
                <li key={offset}>
                    <img src={imageUrl} alt="Page of PDF" />
                </li>
            ))}
        </ul>

    </>);
}
