import React from 'react';
import styled, { keyframes } from 'styled-components';

export interface ProgressProps {
    label: string;
    percent: number | null;
}

const animation = keyframes`
    0%, 100% {
        margin-left: 0;
    }

    50% {
        margin-left: 80%;
    }
`;

const BouncingDiv = styled.div`
    width: 20%;
    animation: ${animation} 7s infinite ease-in-out;
`;

export default function Progress(props: ProgressProps): JSX.Element {
    let percent = null;
    if (props.percent !== null) {
        percent = Math.round(props.percent * 10000) / 100;
    }

    return (<section className='fixed top-0 left-0 right-0 flex p-5 bg-gray-800 text-gray-200'>
        <h1 className='inline flex-initial mr-4 text-lg'> {props.label} </h1>
        <div
            className='flex-grow h-4 mt-1 bg-gray-600'
            title={percent !== null ? `${percent}% complete` : 'unknown percent complete'}
        >
            {percent !== null ?
                <div className='bg-gray-400 h-full' style={{ width: percent.toString() + '%'}}></div> :
                <BouncingDiv className='bg-gray-400 h-full'></BouncingDiv>
            }
        </div>
    </section>)
}