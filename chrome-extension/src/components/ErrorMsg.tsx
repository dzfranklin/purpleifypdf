import React from 'react';
import styled from 'styled-components';
import tw from 'tailwind.macro';

export interface ErrorMsgProps {
    title: string;
    children?: undefined | string | JSX.Element | (JSX.Element | string)[];
}

const MsgBody = styled.div`
    ${tw`leading-tight`}

    & > p, div {
        ${tw`pb-2`}
    }
`;

export default function ErrorMsg(props: ErrorMsgProps): JSX.Element {
    let children = props.children;
    if (children === undefined) {
    } else if (typeof children === 'string') {
        children = (<p> {children} </p>);
    } else if ('length' in children) {
        children = children.map(el => typeof el === 'string' ? <p>{el}</p> : el);
    }

    return (
        <section
            className='max-w-xl m-5 bg-red-200 border-l-4 border-red-500 text-red-700 p-6 pt-8'
            role='alert'
        >
            <h1 className='font-bold text-xl pb-3'> {props.title} </h1>
            <MsgBody> {
                children
            } </MsgBody>
        </section>
    )
}

type ErrorLike = string | null | undefined | Error;
export function formatSource(source: ErrorLike): JSX.Element {
    if (source == null || source === undefined) {
        return <></>;
    }

    let errorSource: string;
    if (typeof source === 'string') {
        errorSource = source;
    } else {
        errorSource = source.stack || source.toString();
    }

    return (
        <div>
            <h2 className="text-base font-semibold pb-1">Caused by:</h2>
            <p className="whitespace-pre-wrap text-xs"> { errorSource } </p>
        </div>
    );
}
