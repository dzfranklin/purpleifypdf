import React, { useState, useEffect } from 'react';
import styled from 'styled-components';
import tw from 'tailwind.macro';

export interface ToggleProps {
    title: string;
    value: boolean;
    onChange: (value: boolean) => void;

    color?: {
        base?: string,
        checked?: string,
        toggle?: string,
        toggleChecked?: string
    };
}

const StyledInput = styled.input`
    ${tw`appearance-none`}

    /* base */

    &::before {
        content: '';
        transition: background-color 1s ease-out;
        ${tw`
        absolute left-0 top-0
        inline-block
        w-12 h-4
        rounded-lg
        outline-none
        bg-gray-300
        `}
    }

    &:checked::before {
        ${tw`bg-gray-500`}
    }

    /* toggle */

    &::after {
        content: '';
        ${tw`
        absolute left-0 top-0 -m-1
        inline-block
        w-6 h-6
        rounded-full
        bg-gray-100
        `}
        transition: margin-left 200ms ease-out;
    }

    &:checked::after {
        margin-left: calc(3rem - 1.5rem);
    }
`;

export default function Toggle(props: ToggleProps): JSX.Element {
    let baseColor = props.color?.base;
    let baseStyle = baseColor ? 'background-color: ' + baseColor : '';
    let checkedColor = props.color?.checked;
    let checkedStyle = checkedColor ? 'background-color: ' + checkedColor : '';
    let toggleColor = props.color?.toggle;
    let toggleStyle = toggleColor ? 'background-color: ' + toggleColor : '';
    let toggleCheckedColor = props.color?.toggleChecked;
    let toggleCheckedStyle = toggleCheckedColor ? 'background-color: ' + toggleCheckedColor : '';

    let uniqueCls = useState('uniqueCls-' + Math.floor(Math.random() * Math.pow(10, 20)).toString(36))[0];

    useEffect(() => {
        const elem = document.createElement('style');
        elem.innerText = `
            .${uniqueCls}::before { ${baseStyle} }
            .${uniqueCls}:checked::before { ${checkedStyle} }
            .${uniqueCls}::after { ${toggleStyle} }
            .${uniqueCls}:checked::after { ${toggleCheckedStyle} }
        `;
        document.head.append(elem);

        return () => { document.head.removeChild(elem) }
    }, [uniqueCls, baseStyle, checkedStyle, toggleStyle, toggleCheckedStyle]);

    return (
        <StyledInput
            type="checkbox"
            title={props.title}
            checked={props.value}
            onChange={props.onChange.bind(null, !props.value)}
            className={uniqueCls}
        ></StyledInput>
    )
}