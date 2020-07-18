import { useState } from 'react';

export default function useStorage<T>(key: string,
    defaultValue: T,
    converter: { load: (s: string) => T, stringify: (v: T) => string }):
    [T, (newValue: T) => void] {
    const persistentStore = (value: T) => localStorage.setItem(key, converter.stringify(value));

    let currentValue: T;
    let rawCurrentValue = localStorage.getItem(key);
    if (rawCurrentValue === null) {
        currentValue = defaultValue;
        persistentStore(currentValue);
    } else {
        currentValue = converter.load(rawCurrentValue);
    }

    const reactStore = useState(currentValue)[1];

    const setter = (newValue: T) => {
        persistentStore(newValue);
        reactStore(newValue);
    }

    return [currentValue, setter];
}