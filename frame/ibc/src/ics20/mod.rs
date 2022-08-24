pub mod context;

use crate::{ChannelIds, Config, Event, Pallet, WeightInfo};
use alloc::{
	format,
	string::{String, ToString},
};
use composable_traits::{
	defi::DeFiComposableConfig,
	xcm::assets::{RemoteAssetRegistryInspect, RemoteAssetRegistryMutate, XcmAssetLocation},
};
use core::{fmt::Formatter, str::FromStr};
use frame_support::weights::Weight;
use ibc::{
	applications::transfer::{
		acknowledgement::{Acknowledgement as Ics20Acknowledgement, ACK_ERR_STR, ACK_SUCCESS_B64},
		is_receiver_chain_source,
		packet::PacketData,
		PrefixedCoin, PrefixedDenom, TracePrefix, VERSION,
	},
	core::{
		ics04_channel::{
			channel::{Counterparty, Order},
			error::Error as Ics04Error,
			msgs::acknowledgement::Acknowledgement,
			packet::Packet,
			Version,
		},
		ics24_host::identifier::{ChannelId, ConnectionId, PortId},
		ics26_routing::context::{Module, ModuleOutputBuilder, OnRecvPacketAck},
	},
	signer::Signer,
};
use ibc_primitives::{CallbackWeight, IbcTrait};
use primitives::currency::CurrencyId;
use sp_std::{boxed::Box, marker::PhantomData};

#[derive(Clone)]
pub struct IbcCallbackHandler<T: Config>(PhantomData<T>);

impl<T: Config> core::fmt::Debug for IbcCallbackHandler<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		write!(f, "ibc-transfer")
	}
}

impl<T: Config> Default for IbcCallbackHandler<T> {
	fn default() -> Self {
		Self(PhantomData::default())
	}
}

impl<T: Config + Send + Sync> Module for IbcCallbackHandler<T>
where
	<T as DeFiComposableConfig>::MayBeAssetId: From<CurrencyId>,
	u32: From<<T as frame_system::Config>::BlockNumber>,
	<T as DeFiComposableConfig>::MayBeAssetId:
		From<<T::AssetRegistry as RemoteAssetRegistryMutate>::AssetId>,
	<T as DeFiComposableConfig>::MayBeAssetId:
		From<<T::AssetRegistry as RemoteAssetRegistryInspect>::AssetId>,
	<T::AssetRegistry as RemoteAssetRegistryInspect>::AssetId:
		From<<T as DeFiComposableConfig>::MayBeAssetId>,
	<T::AssetRegistry as RemoteAssetRegistryMutate>::AssetId:
		From<<T as DeFiComposableConfig>::MayBeAssetId>,
	<T::AssetRegistry as RemoteAssetRegistryInspect>::AssetNativeLocation: From<XcmAssetLocation>,
	<T::AssetRegistry as RemoteAssetRegistryMutate>::AssetNativeLocation: From<XcmAssetLocation>,
	<T as DeFiComposableConfig>::MayBeAssetId: From<<T as assets::Config>::AssetId>,
{
	fn on_chan_open_init(
		&mut self,
		_output: &mut ModuleOutputBuilder,
		_order: Order,
		_connection_hops: &[ConnectionId],
		_port_id: &PortId,
		_channel_id: &ChannelId,
		_counterparty: &Counterparty,
		_version: &Version,
	) -> Result<(), Ics04Error> {
		Ok(())
	}

	fn on_chan_open_try(
		&mut self,
		_output: &mut ModuleOutputBuilder,
		_order: Order,
		_connection_hops: &[ConnectionId],
		_port_id: &PortId,
		_channel_id: &ChannelId,
		_counterparty: &Counterparty,
		_version: &Version,
		_counterparty_version: &Version,
	) -> Result<Version, Ics04Error> {
		Ok(Version::new(VERSION.to_string()))
	}

	fn on_chan_open_ack(
		&mut self,
		_output: &mut ModuleOutputBuilder,
		_port_id: &PortId,
		channel_id: &ChannelId,
		_counterparty_version: &Version,
	) -> Result<(), Ics04Error> {
		let _ = ChannelIds::<T>::try_mutate::<_, (), _>(|channels| {
			channels.push(channel_id.to_string().as_bytes().to_vec());
			Ok(())
		});
		Ok(())
	}

	fn on_chan_open_confirm(
		&mut self,
		_output: &mut ModuleOutputBuilder,
		_port_id: &PortId,
		channel_id: &ChannelId,
	) -> Result<(), Ics04Error> {
		let _ = ChannelIds::<T>::try_mutate::<_, (), _>(|channels| {
			channels.push(channel_id.to_string().as_bytes().to_vec());
			Ok(())
		});
		Ok(())
	}

	fn on_chan_close_init(
		&mut self,
		_output: &mut ModuleOutputBuilder,
		port_id: &PortId,
		channel_id: &ChannelId,
	) -> Result<(), Ics04Error> {
		let _ = ChannelIds::<T>::try_mutate::<_, (), _>(|channels| {
			let rem = channels
				.iter()
				.filter(|chan| chan.as_slice() != channel_id.to_string().as_bytes())
				.cloned()
				.collect();
			*channels = rem;
			Ok(())
		});
		// Remove escrow address for closed channel if it exists
		Pallet::<T>::remove_channel_escrow_address(port_id, *channel_id)
	}

	fn on_chan_close_confirm(
		&mut self,
		_output: &mut ModuleOutputBuilder,
		port_id: &PortId,
		channel_id: &ChannelId,
	) -> Result<(), Ics04Error> {
		let _ = ChannelIds::<T>::try_mutate::<_, (), _>(|channels| {
			let rem = channels
				.iter()
				.filter(|chan| chan.as_slice() != channel_id.to_string().as_bytes())
				.cloned()
				.collect();
			*channels = rem;
			Ok(())
		});
		// Remove escrow address for closed channel if it exists
		Pallet::<T>::remove_channel_escrow_address(port_id, *channel_id)
	}

	fn on_recv_packet(
		&self,
		output: &mut ModuleOutputBuilder,
		packet: &Packet,
		_relayer: &Signer,
	) -> OnRecvPacketAck {
		match Pallet::<T>::on_receive_packet(output, packet) {
			Err(err) => {
				Pallet::<T>::deposit_event(Event::<T>::OnRecvPacketError {
					msg: format!("{:?}", err).as_bytes().to_vec(),
				});
				let packet = packet.clone();
				return OnRecvPacketAck::Nil(Box::new(move |_ctx| {
					Pallet::<T>::write_acknowlegdement(
						&packet,
						format!("{}: {:?}", ACK_ERR_STR, err).as_bytes().to_vec(),
					)
					.map_err(|e| format!("[on_recv_packet] {:#?}", e))
				}))
			},
			Ok(_) => {
				let packet_data: PacketData = serde_json::from_slice(packet.data.as_slice())
					.expect("packet data should deserialize successfully");
				let denom = full_ibc_denom(packet, packet_data.token.clone());
				let prefixed_denom = PrefixedDenom::from_str(&denom).expect("Should not fail");
				let token =
					PrefixedCoin { denom: prefixed_denom, amount: packet_data.token.amount };
				Pallet::<T>::deposit_event(Event::<T>::TokenReceived {
					from: packet_data.sender.to_string().as_bytes().to_vec(),
					to: packet_data.receiver.to_string().as_bytes().to_vec(),
					ibc_denom: denom.as_bytes().to_vec(),
					local_asset_id: Pallet::<T>::ibc_denom_to_asset_id(denom, token),
					amount: packet_data.token.amount.as_u256().as_u128().into(),
				});
				let packet = packet.clone();
				return OnRecvPacketAck::Successful(
					Box::new(Ics20Acknowledgement::success()),
					Box::new(move |_ctx| {
						T::IbcHandler::write_acknowlegdement(
							&packet,
							Ics20Acknowledgement::success().as_ref().to_vec(),
						)
						.map_err(|e| format!("[on_recv_packet] {:#?}", e))
					}),
				)
			},
		};
	}

	fn on_acknowledgement_packet(
		&mut self,
		output: &mut ModuleOutputBuilder,
		packet: &Packet,
		acknowledgement: &Acknowledgement,
		_relayer: &Signer,
	) -> Result<(), Ics04Error> {
		let packet_data: PacketData =
			serde_json::from_slice(packet.data.as_slice()).map_err(|e| {
				Ics04Error::implementation_specific(format!("Failed to decode packet data {:?}", e))
			})?;
		Pallet::<T>::on_ack_packet(output, packet, acknowledgement)
			.map(|_| {
				let ack = String::from_utf8(acknowledgement.as_ref().to_vec())
					.expect("Should be valid acknowledgement");
				if ack.as_bytes() == ACK_SUCCESS_B64 {
					Pallet::<T>::deposit_event(Event::<T>::TokenTransferCompleted {
						from: packet_data.sender.to_string().as_bytes().to_vec(),
						to: packet_data.receiver.to_string().as_bytes().to_vec(),
						ibc_denom: packet_data.token.denom.to_string().as_bytes().to_vec(),
						local_asset_id: Pallet::<T>::ibc_denom_to_asset_id(
							packet_data.token.denom.to_string(),
							packet_data.token.clone(),
						),
						amount: packet_data.token.amount.as_u256().as_u128().into(),
					})
				} else {
					Pallet::<T>::deposit_event(Event::<T>::RecievedAcknowledgementError)
				}
			})
			.map_err(|e| {
				Ics04Error::app_module(format!(
					"[ibc-transfer]: Error processing acknowledgement {:#?}",
					e
				))
			})
	}

	fn on_timeout_packet(
		&mut self,
		output: &mut ModuleOutputBuilder,
		packet: &Packet,
		_relayer: &Signer,
	) -> Result<(), Ics04Error> {
		Pallet::<T>::on_timeout_packet(output, packet).map_err(|e| {
			Ics04Error::app_module(format!(
				"[ibc-transfer]: Error processing timeout packet {:#?}",
				e
			))
		})
	}
}

pub struct WeightHandler<T: Config>(PhantomData<T>);

impl<T: Config> Default for WeightHandler<T> {
	fn default() -> Self {
		Self(PhantomData::default())
	}
}

impl<T: Config> CallbackWeight for WeightHandler<T> {
	fn on_chan_open_init(&self) -> Weight {
		<T as Config>::WeightInfo::on_chan_open_init()
	}

	fn on_chan_open_try(&self) -> Weight {
		<T as Config>::WeightInfo::on_chan_open_try()
	}

	fn on_chan_open_ack(&self, _port_id: &PortId, _channel_id: &ChannelId) -> Weight {
		<T as Config>::WeightInfo::on_chan_open_ack()
	}

	fn on_chan_open_confirm(&self, _port_id: &PortId, _channel_id: &ChannelId) -> Weight {
		<T as Config>::WeightInfo::on_chan_open_confirm()
	}

	fn on_chan_close_init(&self, _port_id: &PortId, _channel_id: &ChannelId) -> Weight {
		<T as Config>::WeightInfo::on_chan_close_init()
	}

	fn on_chan_close_confirm(&self, _port_id: &PortId, _channel_id: &ChannelId) -> Weight {
		<T as Config>::WeightInfo::on_chan_close_confirm()
	}

	fn on_recv_packet(&self, _packet: &Packet) -> Weight {
		<T as Config>::WeightInfo::on_recv_packet()
	}

	fn on_acknowledgement_packet(
		&self,
		_packet: &Packet,
		_acknowledgement: &Acknowledgement,
	) -> Weight {
		<T as Config>::WeightInfo::on_acknowledgement_packet()
	}

	fn on_timeout_packet(&self, _packet: &Packet) -> Weight {
		<T as Config>::WeightInfo::on_timeout_packet()
	}
}

pub fn full_ibc_denom(packet: &Packet, mut token: PrefixedCoin) -> String {
	if is_receiver_chain_source(packet.source_port.clone(), packet.source_channel, &token.denom) {
		let prefix = TracePrefix::new(packet.source_port.clone(), packet.source_channel);

		token.denom.remove_trace_prefix(&prefix);
		token.denom.to_string()
	} else {
		let prefix = TracePrefix::new(packet.destination_port.clone(), packet.destination_channel);

		token.denom.add_trace_prefix(prefix);
		token.denom.to_string()
	}
}
