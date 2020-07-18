import React, { useState, useEffect } from 'react';
import tw from 'tailwind.macro';
import { GoSettings } from 'react-icons/go';

import { Quality, qualityFromString } from '../utils/quality';
import HexColor from '../utils/hexcolor';

export interface ControlsProps {
    backgroundColor: HexColor;
    setBackgroundColor: (backgroundColor: HexColor) => void;

    quality: Quality;
    setQuality: (quality: Quality) => void;
}

export default function Controls(props: ControlsProps): JSX.Element {
    const [showSettings, setShowSettings] = useState(false);

    const [newBackgroundColor, setNewBackgroundColor] = useState(props.backgroundColor.css);
    const [newQuality, setNewQuality] = useState(props.quality.toString());

    useEffect(() => {
        setNewBackgroundColor(props.backgroundColor.css);
    }, [props.backgroundColor]);
    useEffect(() => {
        console.log('setting newQ from props to', props.quality);
        setNewQuality(props.quality.toString());
    }, [props.quality]);

    console.log('newQuality is', newQuality);

    return (
        <div className="fixed bottom-0 left-0 ml-6 mb-8">
            <GoSettings
                title='Settings'
                size='2.2rem'
                style={tw`mt-3 mx-1 p-1 rounded text-purple-500 bg-purple-100`}
                onClick={() => setShowSettings(wasShowing => {
                    if (wasShowing) {
                        if (newQuality !== props.quality.toString()) {
                            console.log("setting new quality", newQuality);
                            props.setQuality(qualityFromString(newQuality));
                        }
                        if (newBackgroundColor !== props.backgroundColor.css) {
                            console.log("setting new bg", newBackgroundColor);
                            props.setBackgroundColor(new HexColor(newBackgroundColor));
                        }
                    }
                    return !wasShowing;
                })}
            ></GoSettings>

            {showSettings && <section>
                <label>Quality</label>
                <select
                    value={newQuality}
                    onChange={evt => setNewQuality(evt.target.value)}>
                    <option value='low'>Low</option>
                    <option value='normal'>Normal</option>
                    <option value='high'>High</option>
                    <option value='extreme'>Extreme</option>
                </select>

                <label>Background color</label>
                <input
                    type='color'
                    value={newBackgroundColor}
                    onChange={evt => setNewBackgroundColor(evt.target.value)}
                />
            </section>}
        </div>
    )
}
