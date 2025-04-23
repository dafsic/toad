package client

import (
	"context"

	pb "github.com/dafsic/toad/proto_go/kraken_grid"
	"go.uber.org/zap"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// KrakenGridClient is the client for connecting to the kraken_grid gRPC service
type KrakenGridClient struct {
	name       string
	logger     *zap.Logger
	conn       *grpc.ClientConn
	grpcAddr   string
	grpcClient pb.KrakenGridServiceClient
}

var _ GrpcClient = (*KrakenGridClient)(nil)

// NewKrakenGridClient creates a new client for the kraken_grid service
func NewKrakenGridClient(logger *zap.Logger, name string, grpcAddr string) (*KrakenGridClient, error) {
	client := &KrakenGridClient{
		name:     name,
		logger:   logger,
		grpcAddr: grpcAddr,
	}

	return client, nil
}

// Name returns the name of the client
func (c *KrakenGridClient) Name() string {
	return c.name
}

// Connect establishes a connection to the gRPC server
func (c *KrakenGridClient) Connect() error {
	conn, err := grpc.NewClient(c.grpcAddr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return err
	}
	c.conn = conn
	c.grpcClient = pb.NewKrakenGridServiceClient(conn)
	c.logger.Info("Connected to kraken_grid gRPC server", zap.String("address", c.grpcAddr))
	return nil
}

// Close terminates the connection to the gRPC server
func (c *KrakenGridClient) Close() error {
	if c.conn != nil {
		c.logger.Info("Closing connection to kraken_grid gRPC server")
		return c.conn.Close()
	}
	return nil
}

// PlaceOrder sends an order request to the gRPC server
func (c *KrakenGridClient) PlaceOrder(ctx context.Context, side string, multiplier int32) (bool, string, error) {
	resp, err := c.grpcClient.PlaceOrder(ctx, &pb.PlaceOrderRequest{
		Side:       side,
		Multiplier: multiplier,
	})

	if err != nil {
		c.logger.Error("gRPC PlaceOrder failed", zap.Error(err))
		return false, "", err
	}

	c.logger.Info("Order placed via gRPC",
		zap.Bool("success", resp.Success),
		zap.String("message", resp.Message))

	return resp.Success, resp.Message, nil
}

func (c *KrakenGridClient) SetBasePrice(ctx context.Context, basePrice float64) (bool, string, error) {
	resp, err := c.grpcClient.SetBasePrice(ctx, &pb.SetBasePriceRequest{
		BasePrice: basePrice,
	})

	if err != nil {
		c.logger.Error("gRPC SetBasePrice failed", zap.Error(err))
		return false, "", err
	}

	c.logger.Info("Base price set via gRPC",
		zap.Bool("success", resp.Success),
		zap.String("message", resp.Message))

	return resp.Success, resp.Message, nil
}
