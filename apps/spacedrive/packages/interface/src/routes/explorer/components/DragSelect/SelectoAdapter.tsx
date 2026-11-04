import {
	forwardRef,
	useEffect,
	useImperativeHandle,
	useRef,
} from "react";
import Selecto, {
	CLASS_NAME,
	EVENTS,
	OPTIONS,
	PROPERTIES,
	type SelectoEvents,
	type SelectoOptions,
} from "selecto";

type SelectoInstance = InstanceType<typeof Selecto>;

export type SelectoHandle = Pick<
	SelectoInstance,
	"checkScroll" | "findSelectableTargets"
>;

type SelectoEventHandlers = {
	[K in Extract<keyof SelectoEvents, string> as `on${Capitalize<K>}`]?: (
		event: SelectoEvents[K],
	) => unknown;
};

export type SelectoAdapterProps = Partial<SelectoOptions> &
	SelectoEventHandlers & {
		className?: string;
	};

function getOptions(props: SelectoAdapterProps): Partial<SelectoOptions> {
	const options: Partial<SelectoOptions> = {};
	const propsByName = props as Record<string, unknown>;
	const optionsByName = options as Record<string, unknown>;

	for (const name of OPTIONS) {
		if (name in propsByName) optionsByName[name] = propsByName[name];
	}

	return options;
}

function eventPropName(name: (typeof EVENTS)[number]) {
	return `on${name[0].toUpperCase()}${name.slice(1)}`;
}

export const SelectoAdapter = forwardRef<SelectoHandle, SelectoAdapterProps>(
	function SelectoAdapter(props, ref) {
		const portalRef = useRef<HTMLDivElement>(null);
		const instanceRef = useRef<SelectoInstance | null>(null);
		const propsRef = useRef(props);
		const options = getOptions(props);
		const optionsRef = useRef(options);
		const previousOptionsRef = useRef(options);
		propsRef.current = props;
		optionsRef.current = options;

		useImperativeHandle(
			ref,
			() => ({
				checkScroll: () => instanceRef.current?.checkScroll(),
				findSelectableTargets: (
					data?: Parameters<SelectoInstance["findSelectableTargets"]>[0],
				) => instanceRef.current?.findSelectableTargets(data) ?? [],
			}),
			[],
		);

		useEffect(() => {
			const portalContainer = portalRef.current;
			if (!portalContainer) return;

			const instance = new Selecto({
				...optionsRef.current,
				portalContainer,
			});
			instanceRef.current = instance;

			for (const name of EVENTS) {
				instance.on(name, (event) => {
					const handler = (propsRef.current as Record<string, unknown>)[
						eventPropName(name)
					] as ((event: unknown) => unknown) | undefined;
					if (handler?.(event) === false) {
						(event as { stop?: () => void }).stop?.();
					}
				});
			}

			return () => {
				instance.destroy();
				instanceRef.current = null;
			};
		}, []);

		useEffect(() => {
			const instance = instanceRef.current;
			if (!instance) return;

			const next = optionsRef.current as Record<string, unknown>;
			const previous = previousOptionsRef.current as Record<string, unknown>;
			const instanceProperties = instance as unknown as Record<string, unknown>;
			for (const name of PROPERTIES) {
				if (previous[name] !== next[name]) instanceProperties[name] = next[name];
			}
			previousOptionsRef.current = optionsRef.current;
		});

		return <div className={CLASS_NAME} ref={portalRef} />;
	},
);

SelectoAdapter.displayName = "SelectoAdapter";
