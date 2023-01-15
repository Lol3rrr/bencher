import { createElementSize } from "@solid-primitives/resize-observer";
import { createMemo, createResource } from "solid-js";
import { createStore } from "solid-js/store";
import LinePlot from "./LinePlot";
import PlotKey from "./PlotKey";

const Plot = (props) => {
  const [perf_active, setPerfActive] = createStore([]);

  const [_perf_active] = createResource(props.perf_data, (json_perf) => {
    const active = [];
    json_perf?.results?.forEach(() => {
      active.push(true);
    });
    setPerfActive(active);
    return active;
  });

  const handlePerfActive = (index: number) => {
    const active = [...perf_active];
    active[index] = !active[index];
    setPerfActive(active);
  };

  let key_ref: HTMLDivElement | undefined;
  const key_size = createElementSize(() => key_ref);
  let plot_ref: HTMLDivElement | undefined;
  const plot_size = createElementSize(() => plot_ref);
  const width = createMemo(() =>
    props.key()
      ? plot_size.width == key_size.width
        ? plot_size.width
        : plot_size.width - key_size.width / 4
      : plot_size.width
  );

  return (
    <div class="container">
      <div
        class={`columns is-reverse-mobile ${props.key() ? "" : "is-vcentered"}`}
      >
        <div
          class={`column ${props.key() ? "is-one-quarter" : "is-narrow"}`}
          ref={(e) => (key_ref = e)}
        >
          <PlotKey
            user={props.user}
            config={props.config}
            path_params={props.path_params}
            branches={props.branches}
            testbeds={props.testbeds}
            benchmarks={props.benchmarks}
            perf_data={props.perf_data}
            key={props.key}
            perf_active={perf_active}
            handleKey={props.handleKey}
            handlePerfActive={handlePerfActive}
          />
        </div>
        <div class="column">
          <nav class="level">
            <div class="level-item" ref={(e) => (plot_ref = e)}>
              <LinePlot
                user={props.user}
                config={props.config}
                path_params={props.path_params}
                perf_data={props.perf_data}
                perf_active={perf_active}
                width={width}
              />
            </div>
          </nav>
        </div>
      </div>
    </div>
  );
};

export default Plot;
