import {
  BoxProps,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
} from "@mui/material";
import { BaseAsset } from "@/components/Atoms";
import React from "react";
import { TableHeader } from "@/defi/types";
import { BoxWrapper } from "../BoxWrapper";
import { useAssetsOverview } from "@/defi/hooks/overview/useAssetsOverview";
import { NoPositionsPlaceholder } from "./NoPositionsPlaceholder";
import { OVERVIEW_ERRORS } from "./errors";
import { usePicaPriceDiscovery } from "@/defi/hooks/overview/usePicaPriceDiscovery";

const tableHeaders: TableHeader[] = [
  {
    header: "Assets",
  },
  {
    header: "Price",
  },
  {
    header: "Amount",
  },
  {
    header: "Value",
  },
];

export const WalletBreakdownBox: React.FC<BoxProps> = ({ ...boxProps }) => {
  const assetsOverview = useAssetsOverview();
  const picaDiscovered = usePicaPriceDiscovery();

  return (
    <BoxWrapper title="Wallet Breakdown" {...boxProps}>
      {assetsOverview.length === 0 && (
        <NoPositionsPlaceholder text={OVERVIEW_ERRORS.NO_ASSETS} />
      )}

      {assetsOverview.length > 0 && (
        <TableContainer>
          <Table>
            <TableHead>
              <TableRow>
                {tableHeaders.map((th) => (
                  <TableCell key={th.header} align="left">
                    {th.header}
                  </TableCell>
                ))}
              </TableRow>
            </TableHead>
            <TableBody>
              {assetsOverview.map((asset) => {
                const ownedValue =
                  asset.getSymbol() === "PICA"
                    ? asset.getBalance().multipliedBy(picaDiscovered)
                    : asset.getBalance().multipliedBy(asset.getPrice());

                const assetPrice = asset.getSymbol() === "PICA" ? picaDiscovered.toFixed(6) : asset.getPrice().toFixed(4)

                return (
                  <TableRow key={asset.getName()}>
                    <TableCell align="left">
                      <BaseAsset
                        label={asset.getSymbol()}
                        icon={asset.getIconUrl()}
                      />
                    </TableCell>
                    <TableCell align="left">
                      <Typography variant="body1">
                        ${assetPrice}
                      </Typography>
                    </TableCell>
                    <TableCell align="left">
                      <Typography variant="body1">
                        {asset.getBalance().toFormat(2)}
                      </Typography>
                    </TableCell>
                    <TableCell align="left">
                      <Typography variant="body1">
                        {new Intl.NumberFormat("en-US", {
                          style: "currency",
                          currency: "USD",
                        }).format(ownedValue.toNumber())}
                      </Typography>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </TableContainer>
      )}
    </BoxWrapper>
  );
};
