import type { Params } from "astro";
import {
	type Accessor,
	For,
	Match,
	type Resource,
	Switch,
	Show,
} from "solid-js";
import { Row } from "../../../config/types";
import { fmtDateTime } from "../../../config/util";
import type { Slug } from "../../../types/bencher";
import { fmtNestedValue, fmtValues } from "../../../util/resource";
import { BACK_PARAM, encodePath, pathname } from "../../../util/url";

export enum TableState {
	LOADING = 0,
	EMPTY = 1,
	OK = 2,
	END = 3,
	ERR = 4,
}

export interface Props {
	config: TableConfig;
	state: Accessor<TableState>;
	tableData: Resource<any[]>;
	start_date: Accessor<undefined | string>;
	end_date: Accessor<undefined | string>;
	search: Accessor<undefined | string>;
	page: Accessor<number>;
	handlePage: (page: number) => void;
}

export interface TableConfig {
	url: (params: Params) => string;
	name: string;
	add?: AddButtonConfig;
	row: RowConfig;
}

const Table = (props: Props) => {
	return (
		<Switch fallback={<p>ERROR: Unknown table state</p>}>
			<Match when={props.state() === TableState.LOADING}>
				{/* biome-ignore lint/complexity/noUselessFragments: loading */}
				<></>
			</Match>

			<Match when={props.state() === TableState.EMPTY}>
				<Show
					when={
						props.config?.add &&
						!props.start_date() &&
						!props.end_date() &&
						!props.search()
					}
					fallback={<p>🐰 No {props.config?.name} found</p>}
				>
					<AddButton config={props.config?.add as AddButtonConfig} />
				</Show>
			</Match>

			<Match when={props.state() === TableState.OK}>
				{" "}
				<div class="pricing-table is-horizontal">
					<For each={props.tableData()}>
						{(datum, _i) => (
							<div class="pricing-plan is-primary">
								<div class="plan-header">
									<RowHeader config={props.config?.row} datum={datum} />
								</div>
								<div class="plan-items">
									<For each={props.config?.row?.items}>
										{(item: ItemConfig, _i) => (
											<div class="plan-item">
												<p style="word-break: break-word;">
													<Switch fallback="-">
														<Match when={item.kind === Row.TEXT}>
															{item.key && datum[item.key]}
														</Match>
														<Match when={item.kind === Row.BOOL}>
															{item.text}:{" "}
															{item.key && datum[item.key] ? "true" : "false"}
														</Match>
														<Match when={item.kind === Row.SELECT}>
															{item.key &&
																item.value?.options.reduce((field, option) => {
																	if (
																		item.key &&
																		datum[item.key] === option.value
																	) {
																		return option.option;
																	} else {
																		return field;
																	}
																}, datum[item.key])}
														</Match>
														<Match when={item.kind === Row.NESTED_TEXT}>
															{item.keys && fmtNestedValue(datum, item.keys)}
														</Match>
													</Switch>
												</p>
											</div>
										)}
									</For>
								</div>
								<div class="plan-footer">
									<RowButton config={props.config?.row?.button} datum={datum} />
								</div>
							</div>
						)}
					</For>
				</div>
			</Match>

			<Match when={props.state() === TableState.END}>
				<div class="box">
					<BackButton
						name={props.config?.name}
						page={props.page}
						handlePage={props.handlePage}
					/>
				</div>
			</Match>

			<Match when={props.state() === TableState.ERR}>
				<LogoutButton />
			</Match>
		</Switch>
	);
};

export interface AddButtonConfig {
	prefix: Element;
	path: (pathname: string) => string;
	text: string;
}

const AddButton = (props: {
	config: AddButtonConfig;
}) => {
	return (
		<>
			<div class="content has-text-centered">{props.config?.prefix}</div>
			<a
				class="button is-primary is-fullwidth"
				href={props.config?.path?.(pathname())}
			>
				{props.config?.text}
			</a>
		</>
	);
};

export interface RowConfig {
	kind?: Row;
	key: string;
	keys?: string[][];
	items: ItemConfig[];
	button: RowsButtonConfig;
}

export interface ItemConfig {
	kind: Row;
	key?: string;
	keys?: string[];
	text?: string;
	value?: { options: { option: string; value: string }[] };
}

export interface RowsButtonConfig {
	text: string;
	path: (pathname: string, datum: { [slug: string]: Slug }) => string;
}

const RowHeader = (props: {
	config: RowConfig;
	datum: Record<string, any>;
}) => {
	if (props.config?.kind === Row.DATE_TIME && props.config?.key) {
		return fmtDateTime(props.datum[props.config?.key]);
	}
	const header = fmtValues(
		props.datum,
		props.config?.key,
		props.config?.keys,
		" | ",
	);
	return <p style="word-break: break-word;">{header}</p>;
};

const RowButton = (props: {
	config: RowsButtonConfig;
	datum: Record<string, any>;
}) => {
	return (
		<a
			class="button is-fullwidth"
			type="button"
			href={`${props.config?.path?.(
				pathname(),
				props.datum,
			)}?${BACK_PARAM}=${encodePath()}`}
		>
			{props.config?.text}
		</a>
	);
};

const BackButton = (props: {
	name: string;
	page: Accessor<number>;
	handlePage: (page: number) => void;
}) => {
	return (
		<button
			class="button is-primary is-fullwidth"
			type="button"
			onClick={(e) => {
				e.preventDefault();
				props.handlePage(props.page() - 1);
			}}
		>
			That's all the {props.name}. Go back.
		</button>
	);
};

const LogoutButton = () => {
	return (
		<>
			<div class="content has-text-centered">Failed to fetch data...</div>
			<div class="columns is-centered">
				<div class="column is-one-third">
					<a class="button is-primary is-fullwidth" href="/auth/logout">
						Log out
					</a>
				</div>
			</div>
		</>
	);
};

export default Table;
