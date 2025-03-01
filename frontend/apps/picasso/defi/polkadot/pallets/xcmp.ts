import { SubstrateNetworkId } from "@/defi/polkadot/types";
import { RelayChainId } from "substrate-react";

export function availableTargetNetwork(
  network: SubstrateNetworkId,
  selectedNetwork: SubstrateNetworkId
) {
  switch (selectedNetwork) {
    case "kusama":
      return network === "picasso";
    case "picasso":
      return ["kusama", "karura", "statemine"].includes(network);
    case "karura":
      return network === "picasso";
    case "statemine":
      return network === "picasso";
  }
}
