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

func (c *KrakenGridClient) PlaceOrder(ctx context.Context, req *pb.PlaceOrderRequest) (*pb.Response, error) {
	return c.grpcClient.PlaceOrder(ctx, req)
}

func (c *KrakenGridClient) Run(ctx context.Context, req *pb.RunRequest) (*pb.Response, error) {
	return c.grpcClient.Run(ctx, req)
}

func (c *KrakenGridClient) Stop(ctx context.Context, req *pb.StopRequest) (*pb.Response, error) {
	return c.grpcClient.Stop(ctx, req)
}

func (c *KrakenGridClient) Status(ctx context.Context, req *pb.StatusRequest) (*pb.StatusResponse, error) {
	return c.grpcClient.Status(ctx, req)
}
